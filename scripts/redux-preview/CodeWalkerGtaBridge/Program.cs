using System.Buffers.Binary;
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

    var exporter = new GlbExporter();
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
            }
        }
        return result;
    }
}

sealed class GlbExporter
{
    private readonly List<Dictionary<string, object>> _bufferViews = [];
    private readonly List<Dictionary<string, object>> _accessors = [];
    private readonly List<Dictionary<string, object>> _materials = [];
    private readonly List<Dictionary<string, object>> _primitives = [];
    private readonly List<byte> _buffer = [];
    private readonly List<string> _warnings = [];
    private int _vertexTotal;
    private int _indexTotal;
    private MinMax _bounds = new();

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
            ["material"] = AddMaterial(modelIndex, geometryIndex)
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

    private int AddMaterial(int modelIndex, int geometryIndex)
    {
        var index = _materials.Count;
        _materials.Add(new Dictionary<string, object>
        {
            ["name"] = $"gta_material_{modelIndex}_{geometryIndex}",
            ["pbrMetallicRoughness"] = new Dictionary<string, object>
            {
                ["baseColorFactor"] = new[] { 0.68, 0.70, 0.72, 1.0 },
                ["metallicFactor"] = 0.35,
                ["roughnessFactor"] = 0.58
            }
        });
        return index;
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
        _bufferViews.Add(new Dictionary<string, object>
        {
            ["buffer"] = 0,
            ["byteOffset"] = offset,
            ["byteLength"] = length,
            ["target"] = target
        });
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
    object boundingBox,
    string[] warnings);
