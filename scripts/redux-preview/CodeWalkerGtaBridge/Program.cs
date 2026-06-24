using System.Buffers.Binary;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using CodeWalker.GameFiles;

var options = CliOptions.Parse(args);
if (options.InputYdr is null || options.OutputGlb is null || options.StatusJson is null)
{
    Console.Error.WriteLine("Usage: CodeWalkerGtaBridge --input-yDR <file.ydr> --output-glb <preview.glb> --status-json <status.json>");
    return 2;
}

var inputYdr = Path.GetFullPath(options.InputYdr);
var outputGlb = Path.GetFullPath(options.OutputGlb);
var statusJson = Path.GetFullPath(options.StatusJson);

Directory.CreateDirectory(Path.GetDirectoryName(outputGlb)!);
Directory.CreateDirectory(Path.GetDirectoryName(statusJson)!);

try
{
    var sourceBytes = File.ReadAllBytes(inputYdr);
    var ydr = new YdrFile();
    ydr.Load(sourceBytes);
    if (ydr.Drawable is null)
    {
        throw new InvalidOperationException("CodeWalker parsed the YDR resource but returned no Drawable.");
    }

    var textureLibrary = TextureLibrary.Load(options.TextureRoot, options.InputYdr);
    var exporter = new GlbExporter(textureLibrary);
    var result = exporter.ExportDrawable(ydr.Drawable, outputGlb);
    var sha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(outputGlb))).ToLowerInvariant();

    WriteJson(statusJson, new
    {
        generatedAt = DateTimeOffset.UtcNow.ToString("O"),
        status = "ready",
        sourcePath = inputYdr,
        outputGlb,
        outputSha256 = sha256,
        result.meshCount,
        result.primitiveCount,
        result.vertexCount,
        result.indexCount,
        result.materialCount,
        result.textureCount,
        result.texturedPrimitiveCount,
        result.boundingBox,
        warnings = result.warnings
    });

    Console.WriteLine($"Wrote {outputGlb}");
    Console.WriteLine($"Meshes: {result.meshCount}, primitives: {result.primitiveCount}, vertices: {result.vertexCount}, indices: {result.indexCount}");
    return 0;
}
catch (Exception ex)
{
    WriteJson(statusJson, new
    {
        generatedAt = DateTimeOffset.UtcNow.ToString("O"),
        status = "failed",
        sourcePath = inputYdr,
        outputGlb,
        error = ex.ToString()
    });
    Console.Error.WriteLine(ex);
    return 1;
}

static void WriteJson(string path, object value)
{
    var json = JsonSerializer.Serialize(value, new JsonSerializerOptions { WriteIndented = true });
    File.WriteAllText(path, json, Encoding.UTF8);
}

sealed class CliOptions
{
    public string? InputYdr { get; private set; }
    public string? OutputGlb { get; private set; }
    public string? StatusJson { get; private set; }
    public string? TextureRoot { get; private set; }

    public static CliOptions Parse(string[] args)
    {
        var result = new CliOptions();
        for (var i = 0; i < args.Length; i++)
        {
            var key = args[i].ToLowerInvariant();
            var value = i + 1 < args.Length ? args[++i] : "";
            switch (key)
            {
                case "--input-ydr":
                case "--input-ydr-file":
                case "--input-ydr-path":
                    result.InputYdr = value;
                    break;
                case "--output-glb":
                    result.OutputGlb = value;
                    break;
                case "--status-json":
                    result.StatusJson = value;
                    break;
                case "--texture-root":
                    result.TextureRoot = value;
                    break;
            }
        }
        return result;
    }
}

sealed class GlbExporter
{
    private readonly TextureLibrary _textures;
    private readonly List<Dictionary<string, object>> _bufferViews = [];
    private readonly List<Dictionary<string, object>> _accessors = [];
    private readonly List<Dictionary<string, object>> _materials = [];
    private readonly List<Dictionary<string, object>> _images = [];
    private readonly List<Dictionary<string, object>> _gltfTextures = [];
    private readonly List<Dictionary<string, object>> _primitives = [];
    private readonly List<byte> _buffer = [];
    private readonly Dictionary<uint, int> _textureImageIndices = [];
    private readonly List<string> _warnings = [];
    private int _vertexTotal;
    private int _indexTotal;
    private int _texturedPrimitiveTotal;
    private MinMax _bounds = new();

    public GlbExporter(TextureLibrary textures)
    {
        _textures = textures;
        foreach (var warning in textures.Warnings)
        {
            _warnings.Add(warning);
        }
    }

    public ExportResult ExportDrawable(Drawable drawable, string outputGlb)
    {
        var models = drawable.AllModels ?? [];
        for (var modelIndex = 0; modelIndex < models.Length; modelIndex++)
        {
            var model = models[modelIndex];
            var geometries = model?.Geometries ?? [];
            for (var geometryIndex = 0; geometryIndex < geometries.Length; geometryIndex++)
            {
                ExportGeometry(geometries[geometryIndex], modelIndex, geometryIndex);
            }
        }

        if (_primitives.Count == 0)
        {
            throw new InvalidOperationException("No supported geometry primitives were found in the Drawable.");
        }

        var gltf = new Dictionary<string, object>
        {
            ["asset"] = new Dictionary<string, object>
            {
                ["version"] = "2.0",
                ["generator"] = "HomeOps CodeWalkerGtaBridge"
            },
            ["scene"] = 0,
            ["scenes"] = new object[]
            {
                new Dictionary<string, object> { ["nodes"] = new[] { 0 } }
            },
            ["nodes"] = new object[]
            {
                new Dictionary<string, object> { ["mesh"] = 0, ["name"] = drawable.Name ?? "gta_drawable" }
            },
            ["meshes"] = new object[]
            {
                new Dictionary<string, object> { ["name"] = drawable.Name ?? "gta_drawable", ["primitives"] = _primitives }
            },
            ["materials"] = _materials,
            ["textures"] = _gltfTextures,
            ["images"] = _images,
            ["buffers"] = new object[]
            {
                new Dictionary<string, object> { ["byteLength"] = _buffer.Count }
            },
            ["bufferViews"] = _bufferViews,
            ["accessors"] = _accessors
        };

        var jsonBytes = Pad4(Encoding.UTF8.GetBytes(JsonSerializer.Serialize(gltf)), 0x20);
        var binBytes = Pad4(_buffer.ToArray(), 0x00);
        using var fs = File.Create(outputGlb);
        using var bw = new BinaryWriter(fs, Encoding.UTF8, leaveOpen: false);
        bw.Write(Encoding.ASCII.GetBytes("glTF"));
        bw.Write(2u);
        bw.Write((uint)(12 + 8 + jsonBytes.Length + 8 + binBytes.Length));
        bw.Write((uint)jsonBytes.Length);
        bw.Write(0x4E4F534Au);
        bw.Write(jsonBytes);
        bw.Write((uint)binBytes.Length);
        bw.Write(0x004E4942u);
        bw.Write(binBytes);

        return new ExportResult(
            meshCount: 1,
            primitiveCount: _primitives.Count,
            vertexCount: _vertexTotal,
            indexCount: _indexTotal,
            materialCount: _materials.Count,
            textureCount: _gltfTextures.Count,
            texturedPrimitiveCount: _texturedPrimitiveTotal,
            boundingBox: _bounds.ToObject(),
            warnings: _warnings.ToArray());
    }

    private void ExportGeometry(DrawableGeometry geometry, int modelIndex, int geometryIndex)
    {
        var vertexData = geometry.VertexData;
        var indexBuffer = geometry.IndexBuffer;
        if (vertexData?.VertexBytes is null || vertexData.VertexCount == 0)
        {
            _warnings.Add($"Skipped geometry {modelIndex}:{geometryIndex}: missing vertex data.");
            return;
        }
        if (indexBuffer?.Indices is null || indexBuffer.Indices.Length < 3)
        {
            _warnings.Add($"Skipped geometry {modelIndex}:{geometryIndex}: missing index data.");
            return;
        }

        var positions = new List<float>(vertexData.VertexCount * 3);
        var normals = new List<float>(vertexData.VertexCount * 3);
        var texcoords = new List<float>(vertexData.VertexCount * 2);
        var primitiveBounds = new MinMax();
        var hasNormal = HasSemantic(vertexData, (int)VertexSemantics.Normal);
        var hasUv = HasSemantic(vertexData, (int)VertexSemantics.TexCoord0);

        for (var i = 0; i < vertexData.VertexCount; i++)
        {
            var p = vertexData.GetVector3(i, (int)VertexSemantics.Position);
            positions.Add(p.X);
            positions.Add(p.Y);
            positions.Add(p.Z);
            _bounds.Include(p.X, p.Y, p.Z);
            primitiveBounds.Include(p.X, p.Y, p.Z);

            if (hasNormal)
            {
                var n = vertexData.GetVector3(i, (int)VertexSemantics.Normal);
                normals.Add(n.X);
                normals.Add(n.Y);
                normals.Add(n.Z);
            }

            if (hasUv)
            {
                var uv = vertexData.GetVector2(i, (int)VertexSemantics.TexCoord0);
                texcoords.Add(uv.X);
                texcoords.Add(1.0f - uv.Y);
            }
        }

        var primitive = new Dictionary<string, object>
        {
            ["attributes"] = new Dictionary<string, object>
            {
                ["POSITION"] = AddFloatAccessor(positions, "VEC3", vertexData.VertexCount, primitiveBounds.MinArray(), primitiveBounds.MaxArray())
            },
            ["indices"] = AddUshortAccessor(indexBuffer.Indices),
            ["material"] = AddMaterial(modelIndex, geometryIndex, geometry.Shader)
        };

        if (hasNormal)
        {
            ((Dictionary<string, object>)primitive["attributes"])["NORMAL"] = AddFloatAccessor(normals, "VEC3", vertexData.VertexCount);
        }
        if (hasUv)
        {
            ((Dictionary<string, object>)primitive["attributes"])["TEXCOORD_0"] = AddFloatAccessor(texcoords, "VEC2", vertexData.VertexCount);
        }

        _vertexTotal += vertexData.VertexCount;
        _indexTotal += indexBuffer.Indices.Length;
        _primitives.Add(primitive);
    }

    private static bool HasSemantic(VertexData vertexData, int semantic)
    {
        return vertexData.Info is not null && ((vertexData.Info.Flags >> semantic) & 1u) == 1u;
    }

    private int AddMaterial(int modelIndex, int geometryIndex, ShaderFX? shader)
    {
        var index = _materials.Count;
        var pbr = new Dictionary<string, object>
        {
            ["baseColorFactor"] = new[] { 0.56, 0.56, 0.56, 1.0 },
            ["metallicFactor"] = 0.18,
            ["roughnessFactor"] = 0.74
        };

        var textureRef = TryResolveDiffuseTexture(shader);
        if (textureRef is not null)
        {
            var textureIndex = AddTexture(textureRef);
            pbr["baseColorTexture"] = new Dictionary<string, object> { ["index"] = textureIndex };
            _texturedPrimitiveTotal++;
        }
        else
        {
            pbr["baseColorFactor"] = new[] { 0.68, 0.70, 0.72, 1.0 };
        }

        _materials.Add(new Dictionary<string, object>
        {
            ["name"] = textureRef is null
                ? $"gta_material_{modelIndex}_{geometryIndex}"
                : $"gta_material_{modelIndex}_{geometryIndex}_{textureRef.Name}",
            ["pbrMetallicRoughness"] = pbr
        });
        return index;
    }

    private GtaTexture? TryResolveDiffuseTexture(ShaderFX? shader)
    {
        var parameters = shader?.ParametersList?.Parameters;
        var hashes = shader?.ParametersList?.Hashes;
        if (parameters is null || hashes is null)
        {
            return null;
        }

        GtaTexture? fallback = null;
        for (var i = 0; i < parameters.Length; i++)
        {
            if (parameters[i].Data is not TextureBase textureBase)
            {
                continue;
            }

            var texture = _textures.Resolve(textureBase.NameHash);
            if (texture is null)
            {
                continue;
            }

            var parameterName = i < hashes.Length ? hashes[i].ToString() : "";
            var textureName = texture.Name.ToLowerInvariant();
            if (parameterName.Equals("DiffuseSampler", StringComparison.OrdinalIgnoreCase) ||
                (textureName.Contains("diff") && !textureName.Contains("spec") && !textureName.Contains("_n")))
            {
                return texture;
            }

            fallback ??= texture;
        }

        return fallback;
    }

    private int AddTexture(GtaTexture texture)
    {
        if (_textureImageIndices.TryGetValue(texture.NameHash, out var existingTextureIndex))
        {
            return existingTextureIndex;
        }

        AlignBuffer();
        var offset = _buffer.Count;
        _buffer.AddRange(texture.PngBytes);
        var view = AddBufferView(offset, texture.PngBytes.Length, 0);
        var imageIndex = _images.Count;
        _images.Add(new Dictionary<string, object>
        {
            ["name"] = texture.Name,
            ["mimeType"] = "image/png",
            ["bufferView"] = view
        });
        var textureIndex = _gltfTextures.Count;
        _gltfTextures.Add(new Dictionary<string, object>
        {
            ["source"] = imageIndex
        });
        _textureImageIndices[texture.NameHash] = textureIndex;
        return textureIndex;
    }

    private int AddFloatAccessor(List<float> values, string type, int count, float[]? min = null, float[]? max = null)
    {
        AlignBuffer();
        var offset = _buffer.Count;
        var bytes = new byte[4];
        foreach (var value in values)
        {
            BinaryPrimitives.WriteSingleLittleEndian(bytes, value);
            _buffer.AddRange(bytes);
        }
        var view = AddBufferView(offset, values.Count * 4, 34962);
        return AddAccessor(view, 5126, count, type, min, max);
    }

    private int AddUshortAccessor(ushort[] values)
    {
        AlignBuffer();
        var offset = _buffer.Count;
        var bytes = new byte[2];
        foreach (var value in values)
        {
            BinaryPrimitives.WriteUInt16LittleEndian(bytes, value);
            _buffer.AddRange(bytes);
        }
        var view = AddBufferView(offset, values.Length * 2, 34963);
        return AddAccessor(view, 5123, values.Length, "SCALAR");
    }

    private int AddBufferView(int offset, int length, int target)
    {
        var index = _bufferViews.Count;
        var view = new Dictionary<string, object>
        {
            ["buffer"] = 0,
            ["byteOffset"] = offset,
            ["byteLength"] = length
        };
        if (target != 0) view["target"] = target;
        _bufferViews.Add(view);
        return index;
    }

    private int AddAccessor(int bufferView, int componentType, int count, string type, float[]? min = null, float[]? max = null)
    {
        var index = _accessors.Count;
        var accessor = new Dictionary<string, object>
        {
            ["bufferView"] = bufferView,
            ["byteOffset"] = 0,
            ["componentType"] = componentType,
            ["count"] = count,
            ["type"] = type
        };
        if (min is not null) accessor["min"] = min;
        if (max is not null) accessor["max"] = max;
        _accessors.Add(accessor);
        return index;
    }

    private void AlignBuffer()
    {
        while ((_buffer.Count % 4) != 0) _buffer.Add(0);
    }

    private static byte[] Pad4(byte[] bytes, byte pad)
    {
        var padded = new byte[(bytes.Length + 3) & ~3];
        Array.Copy(bytes, padded, bytes.Length);
        for (var i = bytes.Length; i < padded.Length; i++) padded[i] = pad;
        return padded;
    }
}

sealed class MinMax
{
    public float MinX = float.PositiveInfinity;
    public float MinY = float.PositiveInfinity;
    public float MinZ = float.PositiveInfinity;
    public float MaxX = float.NegativeInfinity;
    public float MaxY = float.NegativeInfinity;
    public float MaxZ = float.NegativeInfinity;

    public void Include(float x, float y, float z)
    {
        MinX = Math.Min(MinX, x);
        MinY = Math.Min(MinY, y);
        MinZ = Math.Min(MinZ, z);
        MaxX = Math.Max(MaxX, x);
        MaxY = Math.Max(MaxY, y);
        MaxZ = Math.Max(MaxZ, z);
    }

    public float[] MinArray() => [MinX, MinY, MinZ];
    public float[] MaxArray() => [MaxX, MaxY, MaxZ];
    public object ToObject() => new { min = MinArray(), max = MaxArray() };
}

sealed record ExportResult(
    int meshCount,
    int primitiveCount,
    int vertexCount,
    int indexCount,
    int materialCount,
    int textureCount,
    int texturedPrimitiveCount,
    object boundingBox,
    string[] warnings);

sealed class TextureLibrary
{
    private readonly Dictionary<uint, GtaTexture> _textures;
    public string[] Warnings { get; }

    private TextureLibrary(Dictionary<uint, GtaTexture> textures, List<string> warnings)
    {
        _textures = textures;
        Warnings = warnings.ToArray();
    }

    public static TextureLibrary Empty { get; } = new([], []);

    public static TextureLibrary Load(string? textureRoot, string inputYdr)
    {
        var warnings = new List<string>();
        var textures = new Dictionary<uint, GtaTexture>();
        var root = string.IsNullOrWhiteSpace(textureRoot)
            ? Path.GetDirectoryName(inputYdr)
            : Path.GetFullPath(textureRoot);

        if (string.IsNullOrWhiteSpace(root) || !Directory.Exists(root))
        {
            warnings.Add("Texture root was not found; exported GLB uses fallback material colors.");
            return new TextureLibrary(textures, warnings);
        }

        foreach (var ytdPath in Directory.EnumerateFiles(root, "*.ytd", SearchOption.AllDirectories))
        {
            try
            {
                var ytd = new YtdFile();
                ytd.Load(File.ReadAllBytes(ytdPath));
                var items = ytd.TextureDict?.Textures?.data_items;
                if (items is null) continue;

                foreach (var texture in items)
                {
                    if (texture is null || texture.Data?.FullData is null) continue;
                    if (textures.ContainsKey(texture.NameHash)) continue;
                    try
                    {
                        var rgba = TextureDecoder.DecodeRgba(texture);
                        var png = PngWriter.WriteRgba(texture.Width, texture.Height, rgba);
                        textures[texture.NameHash] = new GtaTexture(texture.Name ?? texture.NameHash.ToString("x8"), texture.NameHash, png);
                    }
                    catch (Exception ex)
                    {
                        warnings.Add($"Could not decode texture {texture.Name ?? texture.NameHash.ToString("x8")} from {Path.GetFileName(ytdPath)}: {ex.Message}");
                    }
                }
            }
            catch (Exception ex)
            {
                warnings.Add($"Could not load YTD {ytdPath}: {ex.Message}");
            }
        }

        if (textures.Count == 0)
        {
            warnings.Add("No decodable YTD textures were found; exported GLB uses fallback material colors.");
        }

        return new TextureLibrary(textures, warnings);
    }

    public GtaTexture? Resolve(uint nameHash)
    {
        _textures.TryGetValue(nameHash, out var texture);
        return texture;
    }
}

sealed record GtaTexture(string Name, uint NameHash, byte[] PngBytes);

static class TextureDecoder
{
    public static byte[] DecodeRgba(Texture texture)
    {
        var data = texture.Data.FullData;
        var width = texture.Width;
        var height = texture.Height;
        var rgba = texture.Format switch
        {
            TextureFormat.D3DFMT_DXT1 => DecodeBc1(data, width, height),
            TextureFormat.D3DFMT_DXT3 => DecodeBc2(data, width, height),
            TextureFormat.D3DFMT_DXT5 => DecodeBc3(data, width, height),
            TextureFormat.D3DFMT_A8R8G8B8 => DecodeA8R8G8B8(data, width, height, alpha: true),
            TextureFormat.D3DFMT_X8R8G8B8 => DecodeA8R8G8B8(data, width, height, alpha: false),
            TextureFormat.D3DFMT_A8B8G8R8 => DecodeA8B8G8R8(data, width, height),
            _ => throw new NotSupportedException($"Unsupported texture format {texture.Format}.")
        };
        return ApplyPreviewContrast(rgba);
    }

    private static byte[] ApplyPreviewContrast(byte[] rgba)
    {
        var adjusted = new byte[rgba.Length];
        for (var i = 0; i + 3 < rgba.Length; i += 4)
        {
            adjusted[i + 0] = AdjustChannel(rgba[i + 0]);
            adjusted[i + 1] = AdjustChannel(rgba[i + 1]);
            adjusted[i + 2] = AdjustChannel(rgba[i + 2]);
            adjusted[i + 3] = rgba[i + 3];
        }
        return adjusted;
    }

    private static byte AdjustChannel(byte value)
    {
        var normalized = value / 255.0;
        var contrasted = ((normalized - 0.5) * 1.35 + 0.5) * 0.68;
        return (byte)Math.Round(Math.Clamp(contrasted, 0, 1) * 255);
    }

    private static byte[] DecodeA8R8G8B8(byte[] data, int width, int height, bool alpha)
    {
        var rgba = new byte[width * height * 4];
        for (var i = 0; i < width * height && i * 4 + 3 < data.Length; i++)
        {
            var src = i * 4;
            var dst = i * 4;
            rgba[dst + 0] = data[src + 2];
            rgba[dst + 1] = data[src + 1];
            rgba[dst + 2] = data[src + 0];
            rgba[dst + 3] = alpha ? data[src + 3] : (byte)255;
        }
        return rgba;
    }

    private static byte[] DecodeA8B8G8R8(byte[] data, int width, int height)
    {
        var rgba = new byte[width * height * 4];
        for (var i = 0; i < width * height && i * 4 + 3 < data.Length; i++)
        {
            var src = i * 4;
            var dst = i * 4;
            rgba[dst + 0] = data[src + 0];
            rgba[dst + 1] = data[src + 1];
            rgba[dst + 2] = data[src + 2];
            rgba[dst + 3] = data[src + 3];
        }
        return rgba;
    }

    private static byte[] DecodeBc1(byte[] data, int width, int height)
    {
        var rgba = new byte[width * height * 4];
        var offset = 0;
        for (var by = 0; by < height; by += 4)
        {
            for (var bx = 0; bx < width; bx += 4)
            {
                if (offset + 8 > data.Length) return rgba;
                DecodeColorBlock(data, offset, null, rgba, width, height, bx, by, bc1Alpha: true);
                offset += 8;
            }
        }
        return rgba;
    }

    private static byte[] DecodeBc2(byte[] data, int width, int height)
    {
        var rgba = new byte[width * height * 4];
        var offset = 0;
        for (var by = 0; by < height; by += 4)
        {
            for (var bx = 0; bx < width; bx += 4)
            {
                if (offset + 16 > data.Length) return rgba;
                var alpha = new byte[16];
                for (var i = 0; i < 16; i++)
                {
                    var packed = data[offset + i / 2];
                    alpha[i] = (byte)(((i & 1) == 0 ? packed & 0x0F : packed >> 4) * 17);
                }
                DecodeColorBlock(data, offset + 8, alpha, rgba, width, height, bx, by, bc1Alpha: false);
                offset += 16;
            }
        }
        return rgba;
    }

    private static byte[] DecodeBc3(byte[] data, int width, int height)
    {
        var rgba = new byte[width * height * 4];
        var offset = 0;
        for (var by = 0; by < height; by += 4)
        {
            for (var bx = 0; bx < width; bx += 4)
            {
                if (offset + 16 > data.Length) return rgba;
                var alpha = DecodeBc3Alpha(data, offset);
                DecodeColorBlock(data, offset + 8, alpha, rgba, width, height, bx, by, bc1Alpha: false);
                offset += 16;
            }
        }
        return rgba;
    }

    private static byte[] DecodeBc3Alpha(byte[] data, int offset)
    {
        Span<byte> palette = stackalloc byte[8];
        palette[0] = data[offset];
        palette[1] = data[offset + 1];
        if (palette[0] > palette[1])
        {
            for (var i = 1; i <= 6; i++) palette[i + 1] = (byte)(((7 - i) * palette[0] + i * palette[1]) / 7);
        }
        else
        {
            for (var i = 1; i <= 4; i++) palette[i + 1] = (byte)(((5 - i) * palette[0] + i * palette[1]) / 5);
            palette[6] = 0;
            palette[7] = 255;
        }

        ulong bits = 0;
        for (var i = 0; i < 6; i++) bits |= (ulong)data[offset + 2 + i] << (8 * i);
        var alpha = new byte[16];
        for (var i = 0; i < 16; i++)
        {
            alpha[i] = palette[(int)((bits >> (3 * i)) & 0x7)];
        }
        return alpha;
    }

    private static void DecodeColorBlock(byte[] data, int offset, byte[]? alpha, byte[] rgba, int width, int height, int bx, int by, bool bc1Alpha)
    {
        var c0 = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(offset, 2));
        var c1 = BinaryPrimitives.ReadUInt16LittleEndian(data.AsSpan(offset + 2, 2));
        Span<byte> colors = stackalloc byte[16];
        WriteRgb565(colors, 0, c0, 255);
        WriteRgb565(colors, 4, c1, 255);
        if (c0 > c1 || !bc1Alpha)
        {
            for (var i = 0; i < 3; i++)
            {
                colors[8 + i] = (byte)((2 * colors[i] + colors[4 + i]) / 3);
                colors[12 + i] = (byte)((colors[i] + 2 * colors[4 + i]) / 3);
            }
            colors[11] = colors[15] = 255;
        }
        else
        {
            for (var i = 0; i < 3; i++) colors[8 + i] = (byte)((colors[i] + colors[4 + i]) / 2);
            colors[11] = 255;
            colors[12] = colors[13] = colors[14] = colors[15] = 0;
        }

        var indices = BinaryPrimitives.ReadUInt32LittleEndian(data.AsSpan(offset + 4, 4));
        for (var y = 0; y < 4; y++)
        {
            for (var x = 0; x < 4; x++)
            {
                var px = bx + x;
                var py = by + y;
                if (px >= width || py >= height) continue;
                var sourceIndex = y * 4 + x;
                var ci = (int)((indices >> (2 * sourceIndex)) & 0x3) * 4;
                var dst = (py * width + px) * 4;
                rgba[dst + 0] = colors[ci + 0];
                rgba[dst + 1] = colors[ci + 1];
                rgba[dst + 2] = colors[ci + 2];
                rgba[dst + 3] = alpha?[sourceIndex] ?? colors[ci + 3];
            }
        }
    }

    private static void WriteRgb565(Span<byte> colors, int offset, ushort value, byte alpha)
    {
        var r = (value >> 11) & 0x1F;
        var g = (value >> 5) & 0x3F;
        var b = value & 0x1F;
        colors[offset + 0] = (byte)((r << 3) | (r >> 2));
        colors[offset + 1] = (byte)((g << 2) | (g >> 4));
        colors[offset + 2] = (byte)((b << 3) | (b >> 2));
        colors[offset + 3] = alpha;
    }
}

static class PngWriter
{
    private static readonly byte[] Signature = [137, 80, 78, 71, 13, 10, 26, 10];

    public static byte[] WriteRgba(int width, int height, byte[] rgba)
    {
        using var output = new MemoryStream();
        output.Write(Signature);
        WriteChunk(output, "IHDR", BuildIhdr(width, height));
        WriteChunk(output, "IDAT", Compress(BuildScanlines(width, height, rgba)));
        WriteChunk(output, "IEND", []);
        return output.ToArray();
    }

    private static byte[] BuildIhdr(int width, int height)
    {
        var ihdr = new byte[13];
        BinaryPrimitives.WriteUInt32BigEndian(ihdr.AsSpan(0, 4), (uint)width);
        BinaryPrimitives.WriteUInt32BigEndian(ihdr.AsSpan(4, 4), (uint)height);
        ihdr[8] = 8;
        ihdr[9] = 6;
        return ihdr;
    }

    private static byte[] BuildScanlines(int width, int height, byte[] rgba)
    {
        var scanlines = new byte[(width * 4 + 1) * height];
        for (var y = 0; y < height; y++)
        {
            var row = y * (width * 4 + 1);
            scanlines[row] = 0;
            Buffer.BlockCopy(rgba, y * width * 4, scanlines, row + 1, width * 4);
        }
        return scanlines;
    }

    private static byte[] Compress(byte[] data)
    {
        using var output = new MemoryStream();
        using (var zlib = new ZLibStream(output, CompressionLevel.Fastest, leaveOpen: true))
        {
            zlib.Write(data);
        }
        return output.ToArray();
    }

    private static void WriteChunk(Stream output, string type, byte[] data)
    {
        Span<byte> length = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(length, (uint)data.Length);
        output.Write(length);
        var typeBytes = Encoding.ASCII.GetBytes(type);
        output.Write(typeBytes);
        output.Write(data);
        var crc = Crc32(typeBytes, data);
        Span<byte> crcBytes = stackalloc byte[4];
        BinaryPrimitives.WriteUInt32BigEndian(crcBytes, crc);
        output.Write(crcBytes);
    }

    private static uint Crc32(byte[] type, byte[] data)
    {
        var crc = 0xffffffffu;
        foreach (var b in type) crc = Update(crc, b);
        foreach (var b in data) crc = Update(crc, b);
        return ~crc;
    }

    private static uint Update(uint crc, byte b)
    {
        crc ^= b;
        for (var i = 0; i < 8; i++)
        {
            crc = (crc & 1) != 0 ? 0xedb88320u ^ (crc >> 1) : crc >> 1;
        }
        return crc;
    }
}
