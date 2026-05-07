using System.Diagnostics;
using System.Runtime.InteropServices;

const int WarmupIterations = 8;
const int MeasureIterations = 64;
var debugNative = args.Any(arg => arg == "--debug-native");
var healthOnly = args.Any(arg => arg == "--health");

var outputArg = args.FirstOrDefault(arg => arg != "--debug-native" && arg != "--health");
var output = outputArg is not null
    ? outputArg
    : Path.GetFullPath(Path.Combine("experiments", "generated", "realtimecsg-cpp-perf-latest.jsonl"));

Directory.CreateDirectory(Path.GetDirectoryName(output)!);
Native.RegisterNoopUnityMethods();
if (healthOnly)
{
    using var healthWriter = new StreamWriter(output, false);
    healthWriter.WriteLine(RunHealthProbe());
    return;
}

var cases = new PerfCase[]
{
    new("single_center_cut", 2, SingleCenterCut),
    new("room_grid_8x8_doors", 192, RoomGrid8x8Doors),
    new("rotated_cut_stack_64", 65, RotatedCutStack64),
    new("common_box_chain_64", 64, CommonBoxChain64),
    new("distant_cutters_512", 513, DistantCutters512),
};

using var writer = new StreamWriter(output, false);
foreach (var perfCase in cases)
{
    writer.WriteLine(RunCase(perfCase));
}

string RunCase(PerfCase perfCase)
{
    var timings = new List<long>(MeasureIterations);
    var triangles = 0;
    var vertices = 0;
    var meshDescriptions = 0;

    Native.ClearAllNodes();
    var tree = perfCase.Build();
    if (tree == 0)
        throw new InvalidOperationException($"Failed to build native RealtimeCSG tree for {perfCase.Name}");

    Native.RebuildAll();
    if (debugNative)
        DumpTree(perfCase.Name, tree);
    if (Native.GetBrushCount() != perfCase.Brushes)
        throw new InvalidOperationException($"Native brush count mismatch for {perfCase.Name}: expected {perfCase.Brushes}, got {Native.GetBrushCount()}, nodes={Native.GetNodeCount()}, trees={Native.GetTreeCount()}, brushMeshes={Native.GetBrushMeshCount()}");
    for (var i = 0; i < WarmupIterations; i++)
        MeasureOnce(tree, out triangles, out vertices, out meshDescriptions);
    if (meshDescriptions == 0)
    {
        DumpTree(perfCase.Name, tree);
        throw new InvalidOperationException($"Native generated no mesh descriptions for {perfCase.Name}: nodes={Native.GetNodeCount()}, brushes={Native.GetBrushCount()}, trees={Native.GetTreeCount()}, brushMeshes={Native.GetBrushMeshCount()}, treeBrushes={Native.GetNumberOfBrushesInTree(tree)}, treeChildren={Native.GetChildNodeCount(tree)}, treeDirty={Native.IsNodeDirty(tree)}");
    }

    for (var i = 0; i < MeasureIterations; i++)
    {
        var stopwatch = Stopwatch.StartNew();
        MeasureOnce(tree, out triangles, out vertices, out meshDescriptions);
        stopwatch.Stop();
        timings.Add(stopwatch.ElapsedTicks);
    }
    Native.ClearAllNodes();

    timings.Sort();
    var mean = timings.Sum() / timings.Count;
    return
        $"{{\"kernel\":\"realtime_csg_cpp\",\"scenario\":\"{perfCase.Name}\",\"brushes\":{perfCase.Brushes},\"iterations\":{MeasureIterations},\"warmup_iterations\":{WarmupIterations},\"mean_ns\":{TicksToNs(mean)},\"min_ns\":{TicksToNs(timings[0])},\"p50_ns\":{TicksToNs(timings[(timings.Count - 1) * 50 / 100])},\"p95_ns\":{TicksToNs(timings[(timings.Count - 1) * 95 / 100])},\"max_ns\":{TicksToNs(timings[^1])},\"triangles\":{triangles},\"vertices\":{vertices},\"mesh_descriptions\":{meshDescriptions}}}";
}

static string RunHealthProbe()
{
    Native.ClearAllNodes();
    var tree = TreeFrom(() => new[]
    {
        Brush("native_health_cube", Vec3.Zero, Vec3.Splat(4), Mat4.Identity, Operation.Additive),
    });
    Native.RebuildAll();
    var updateOk = Native.UpdateAllTreeMeshes();
    var child = Native.GetChildNodeAtIndex(tree, 0);
    var bounds = new Aabb();
    var boundsOk = Native.GetBrushBounds(child, ref bounds);
    var outlineOk = Native.GetBrushOutlineSizes(child, out var outlineVertices, out var visibleOuter, out var visibleInner, out var invisibleOuter, out var invisibleInner, out var invalid);
    var rayStart = new Vec3(-8, 0, 0);
    var rayEnd = new Vec3(8, 0, 0);
    var matrix = Mat4.Identity;
    var rayHits = Native.RayCastIntoTreeMultiCount(tree, ref rayStart, ref rayEnd, ref matrix, 0, false, IntPtr.Zero, 0);
    var meshTypes = MeshQuery.RenderOnlyTypes;
    var meshTypesHandle = GCHandle.Alloc(meshTypes, GCHandleType.Pinned);
    var generated = Native.GenerateMeshDescriptions(tree, meshTypes.Length, meshTypesHandle.AddrOfPinnedObject(), VertexChannelFlags.All, out var meshDescriptions);
    meshTypesHandle.Free();
    var heartbeat = updateOk && Native.GetTreeEnabled(tree) && Native.GetChildNodeCount(tree) == 1 && boundsOk && outlineOk && rayHits > 0;
    var meshExtractionReady = generated && meshDescriptions > 0;
    var json =
        $"{{\"kernel\":\"realtime_csg_cpp\",\"scenario\":\"native_health_cube\",\"heartbeat\":{JsonBool(heartbeat)},\"mesh_extraction_ready\":{JsonBool(meshExtractionReady)},\"update_ok\":{JsonBool(updateOk)},\"tree\":{tree},\"tree_enabled\":{JsonBool(Native.GetTreeEnabled(tree))},\"nodes\":{Native.GetNodeCount()},\"brushes\":{Native.GetBrushCount()},\"trees\":{Native.GetTreeCount()},\"brush_meshes\":{Native.GetBrushMeshCount()},\"tree_children\":{Native.GetChildNodeCount(tree)},\"tree_brushes\":{Native.GetNumberOfBrushesInTree(tree)},\"tree_dirty\":{JsonBool(Native.IsNodeDirty(tree))},\"child_type\":{Native.GetTypeOfNode(child)},\"child_tree\":{Native.GetTreeOfNode(child)},\"child_mesh\":{Native.GetBrushMeshID(child)},\"child_operation\":{Native.GetBrushOperationType(child)},\"bounds_ok\":{JsonBool(boundsOk)},\"bounds_min\":[{bounds.MinX},{bounds.MinY},{bounds.MinZ}],\"bounds_max\":[{bounds.MaxX},{bounds.MaxY},{bounds.MaxZ}],\"outline_ok\":{JsonBool(outlineOk)},\"outline_vertices\":{outlineVertices},\"visible_outer_lines\":{visibleOuter},\"visible_inner_lines\":{visibleInner},\"invisible_outer_lines\":{invisibleOuter},\"invisible_inner_lines\":{invisibleInner},\"invalid_lines\":{invalid},\"raycast_hits\":{rayHits},\"mesh_descriptions_generated\":{JsonBool(generated)},\"mesh_descriptions\":{meshDescriptions}}}";
    Native.ClearAllNodes();
    return json;
}

static void DumpTree(string name, int tree)
{
    Native.RebuildAll();
    var updated = Native.UpdateAllTreeMeshes();
    Console.Error.WriteLine($"native debug {name}: updated={updated} tree={tree} type={Native.GetTypeOfNode(tree)} enabled={Native.GetTreeEnabled(tree)} children={Native.GetChildNodeCount(tree)} dirty={Native.IsNodeDirty(tree)} nodes={Native.GetNodeCount()} brushes={Native.GetBrushCount()} trees={Native.GetTreeCount()} brushMeshes={Native.GetBrushMeshCount()}");
    for (var i = 0; i < Native.GetChildNodeCount(tree); i++)
    {
        var child = Native.GetChildNodeAtIndex(tree, i);
        var bounds = new Aabb();
        var boundsOk = Native.GetBrushBounds(child, ref bounds);
        var outlineOk = Native.GetBrushOutlineSizes(child, out var outlineVertices, out var visibleOuter, out var visibleInner, out var invisibleOuter, out var invisibleInner, out var invalid);
        Console.Error.WriteLine($"  child[{i}] id={child} type={Native.GetTypeOfNode(child)} tree={Native.GetTreeOfNode(child)} mesh={Native.GetBrushMeshID(child)} op={Native.GetBrushOperationType(child)} dirty={Native.IsNodeDirty(child)} boundsOk={boundsOk} bounds=({bounds.MinX},{bounds.MinY},{bounds.MinZ})..({bounds.MaxX},{bounds.MaxY},{bounds.MaxZ}) outlineOk={outlineOk} outlineVertices={outlineVertices} visibleOuter={visibleOuter} visibleInner={visibleInner} invisibleOuter={invisibleOuter} invisibleInner={invisibleInner} invalid={invalid}");
    }
    var rayStart = new Vec3(-8, 0, 0);
    var rayEnd = new Vec3(8, 0, 0);
    var matrix = Mat4.Identity;
    var rayHits = Native.RayCastIntoTreeMultiCount(tree, ref rayStart, ref rayEnd, ref matrix, 0, false, IntPtr.Zero, 0);
    Console.Error.WriteLine($"  raycast hits={rayHits}");
    ProbeGeneratedMesh(tree);
}

static void ProbeGeneratedMesh(int tree)
{
    var indices = new int[8192];
    var positions = new Vec3[8192];
    var indexHandle = GCHandle.Alloc(indices, GCHandleType.Pinned);
    var positionHandle = GCHandle.Alloc(positions, GCHandleType.Pinned);
    var ok = Native.GetGeneratedMesh(tree, 0, 0, indices.Length, indexHandle.AddrOfPinnedObject(), positions.Length, positionHandle.AddrOfPinnedObject(), IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, out var center, out var size);
    positionHandle.Free();
    indexHandle.Free();
    Console.Error.WriteLine($"  direct mesh probe ok={ok} boundsCenter=({center.X},{center.Y},{center.Z}) boundsSize=({size.X},{size.Y},{size.Z}) firstIndex={indices[0]} firstPos=({positions[0].X},{positions[0].Y},{positions[0].Z})");
}

static long TicksToNs(long ticks) => (long)(ticks * (1_000_000_000.0 / Stopwatch.Frequency));
static string JsonBool(bool value) => value ? "true" : "false";

static void MeasureOnce(int tree, out int triangles, out int vertices, out int meshDescriptions)
{
    Native.UpdateAllTreeMeshes();

    var meshTypes = MeshQuery.RenderOnlyTypes;
    var meshTypesHandle = GCHandle.Alloc(meshTypes, GCHandleType.Pinned);
    var generated = Native.GenerateMeshDescriptions(tree, meshTypes.Length, meshTypesHandle.AddrOfPinnedObject(), VertexChannelFlags.All, out meshDescriptions);
    if (!generated || meshDescriptions == 0)
    {
        meshTypesHandle.Free();
        triangles = 0;
        vertices = 0;
        return;
    }
    meshTypesHandle.Free();

    var descriptions = new GeneratedMeshDescription[meshDescriptions];
    var descriptionsHandle = GCHandle.Alloc(descriptions, GCHandleType.Pinned);
    var ok = Native.GetMeshDescriptions(tree, meshDescriptions, descriptionsHandle.AddrOfPinnedObject());
    descriptionsHandle.Free();
    if (!ok)
    {
        triangles = 0;
        vertices = 0;
        return;
    }

    triangles = 0;
    vertices = 0;
    foreach (var description in descriptions)
    {
        if (description.VertexCount <= 0 || description.IndexCount <= 0)
            continue;

        var indices = new int[description.IndexCount];
        var positions = new Vec3[description.VertexCount];
        var indexHandle = GCHandle.Alloc(indices, GCHandleType.Pinned);
        var positionHandle = GCHandle.Alloc(positions, GCHandleType.Pinned);
        ok = Native.GetGeneratedMesh(
            tree,
            description.MeshQueryIndex,
            description.SubMeshQueryIndex,
            description.IndexCount,
            indexHandle.AddrOfPinnedObject(),
            description.VertexCount,
            positionHandle.AddrOfPinnedObject(),
            IntPtr.Zero,
            IntPtr.Zero,
            IntPtr.Zero,
            out _,
            out _);
        positionHandle.Free();
        indexHandle.Free();
        if (ok)
        {
            triangles += description.IndexCount / 3;
            vertices += description.VertexCount;
        }
    }
}

static int SingleCenterCut()
{
    return TreeFrom(() => new[]
    {
        Brush("source", Vec3.Zero, Vec3.Splat(4), Mat4.Identity, Operation.Additive),
        Brush("void", Vec3.Zero, Vec3.Splat(2), Mat4.Identity, Operation.Subtractive),
    });
}

static int RoomGrid8x8Doors()
{
    var nodes = new List<int>();
    const float cell = 6.0f;
    for (var y = 0; y < 8; y++)
    for (var x = 0; x < 8; x++)
    {
        var center = new Vec3((x - 3.5f) * cell, (y - 3.5f) * cell, 0);
        nodes.Add(Brush("floor", center + new Vec3(0, 0, -0.1f), new Vec3(5.6f, 5.6f, 0.2f), Mat4.Identity, Operation.Additive));
        nodes.Add(Brush("north_wall", center + new Vec3(0, 2.8f, 1.5f), new Vec3(5.6f, 0.25f, 3.0f), Mat4.Identity, Operation.Additive));
        nodes.Add(Brush("door", center + new Vec3(0, 2.8f, 1.0f), new Vec3(1.2f, 0.5f, 2.0f), Mat4.Identity, Operation.Subtractive));
    }
    return Tree(nodes.ToArray());
}

static int RotatedCutStack64()
{
    var nodes = new List<int> { Brush("slab", Vec3.Zero, new Vec3(32, 32, 4), Mat4.Identity, Operation.Additive) };
    for (var index = 0; index < 64; index++)
    {
        var angle = index * 0.173f;
        var radius = 11.0f + (index % 7) * 0.35f;
        var center = new Vec3(MathF.Cos(angle) * radius, MathF.Sin(angle) * radius, 0);
        nodes.Add(Brush("rotated_void", center, new Vec3(1.0f + (index % 3) * 0.3f, 8.0f, 5.0f), Mat4.RotationZ(angle), Operation.Subtractive));
    }
    return Tree(nodes.ToArray());
}

static int CommonBoxChain64()
{
    var nodes = new List<int> { Brush("source", Vec3.Zero, new Vec3(32, 16, 8), Mat4.Identity, Operation.Additive) };
    for (var index = 1; index < 64; index++)
    {
        var t = index / 63.0f;
        var center = new Vec3((t - 0.5f) * 6.0f, MathF.Sin(t * MathF.Tau) * 1.5f, 0);
        nodes.Add(Brush("common", center, new Vec3(30.0f - t * 8.0f, 14.0f, 7.0f), Mat4.RotationZ(t * 0.2f), Operation.Intersecting));
    }
    return Tree(nodes.ToArray());
}

static int DistantCutters512()
{
    var nodes = new List<int> { Brush("source", Vec3.Zero, Vec3.Splat(8), Mat4.Identity, Operation.Additive) };
    for (var index = 0; index < 512; index++)
    {
        var row = index / 32;
        var col = index % 32;
        nodes.Add(Brush("far_void", new Vec3(1000.0f + col * 4.0f, 1000.0f + row * 4.0f, 0), Vec3.Splat(1), Mat4.Identity, Operation.Subtractive));
    }
    return Tree(nodes.ToArray());
}

static int Tree(params int[] children)
{
    return TreeFrom(() => children);
}

static int TreeFrom(Func<int[]> buildChildren)
{
    var children = buildChildren();
    if (!Native.GenerateTree(0, out var tree))
        return 0;
    if (!Native.SetTreeEnabled(tree, true) || !Native.GetTreeEnabled(tree))
        throw new InvalidOperationException($"SetTreeEnabled failed for tree {tree}");
    if (!InsertChildNodes(tree, children))
        throw new InvalidOperationException($"InsertChildNodes failed for tree {tree} with {children.Length} children");
    foreach (var child in children)
        Native.SetDirty(child);
    Native.SetDirty(tree);
    return tree;
}

static int Brush(string name, Vec3 center, Vec3 size, Mat4 rotation, Operation operation)
{
    var mesh = Cube(size);
    var meshId = CreateBrushMesh(mesh, name.GetHashCode());
    if (meshId == 0 || !Native.GenerateBrush(name.GetHashCode(), out var brush))
        throw new InvalidOperationException($"Failed to create brush {name}: meshId={meshId}");
    if (!Native.IsBrushMeshIDValid(meshId))
        throw new InvalidOperationException($"Native rejected brush mesh {name}/{meshId}");
    var transform = rotation.WithTranslation(center);
    if (!Native.SetNodeLocalTransformation(brush, ref transform))
        throw new InvalidOperationException($"SetNodeLocalTransformation failed for brush {name}/{brush}");
    if (!Native.SetBrushMeshID(brush, meshId))
        throw new InvalidOperationException($"SetBrushMeshID failed for brush {name}/{brush}, mesh={meshId}");
    if (operation != Operation.Additive && !Native.SetBrushOperationType(brush, operation))
        throw new InvalidOperationException($"SetBrushOperationType failed for brush {name}/{brush}, op={operation}");
    if (Native.GetBrushMeshID(brush) != meshId)
        throw new InvalidOperationException($"Brush {name}/{brush} did not retain mesh id {meshId}");
    Native.SetDirty(brush);
    return brush;
}

static bool InsertChildNodes(int node, int[] children)
{
    var handle = GCHandle.Alloc(children, GCHandleType.Pinned);
    var ok = Native.InsertChildNodeRange(node, 0, children.Length, handle.AddrOfPinnedObject());
    handle.Free();
    return ok;
}

static int CreateBrushMesh(BrushMesh mesh, int userId)
{
    var vh = GCHandle.Alloc(mesh.Vertices, GCHandleType.Pinned);
    var eh = GCHandle.Alloc(mesh.HalfEdges, GCHandleType.Pinned);
    var ph = GCHandle.Alloc(mesh.Polygons, GCHandleType.Pinned);
    var id = Native.CreateBrushMesh(userId, mesh.Vertices.Length, vh.AddrOfPinnedObject(), mesh.HalfEdges.Length, eh.AddrOfPinnedObject(), mesh.Polygons.Length, ph.AddrOfPinnedObject());
    ph.Free();
    eh.Free();
    vh.Free();
    return id;
}

static BrushMesh Cube(Vec3 size)
{
    var min = size * -0.5f;
    var max = size * 0.5f;
    var layers = new SurfaceLayers { LayerUsage = LayerUsageFlags.RenderReceiveCastShadows | LayerUsageFlags.Collidable };
    var surfaces = new[]
    {
        new SurfaceDescription { UV0 = new UvMatrix(new Vec4(-1, 0,  0, -min.X), new Vec4(0, 1,  0,  min.Y)) },
        new SurfaceDescription { UV0 = new UvMatrix(new Vec4( 1, 0,  0,  min.X), new Vec4(0, 1,  0,  min.Y)) },
        new SurfaceDescription { UV0 = new UvMatrix(new Vec4( 0, 0,  1,  min.Z), new Vec4(0, 1,  0,  min.Y)) },
        new SurfaceDescription { UV0 = new UvMatrix(new Vec4( 0, 0, -1, -min.Z), new Vec4(0, 1,  0,  min.Y)) },
        new SurfaceDescription { UV0 = new UvMatrix(new Vec4(-1, 0,  0, -min.X), new Vec4(0, 0, -1, -min.Z)) },
        new SurfaceDescription { UV0 = new UvMatrix(new Vec4( 1, 0,  0,  min.X), new Vec4(0, 0, -1, -min.Z)) },
    };
    return new BrushMesh(
        new[]
        {
            new Vec3(min.X, min.Y, min.Z), new Vec3(min.X, max.Y, min.Z),
            new Vec3(max.X, max.Y, min.Z), new Vec3(max.X, min.Y, min.Z),
            new Vec3(min.X, min.Y, max.Z), new Vec3(max.X, min.Y, max.Z),
            new Vec3(max.X, max.Y, max.Z), new Vec3(min.X, max.Y, max.Z),
        },
        new[]
        {
            new HalfEdge(0,17), new HalfEdge(1,8), new HalfEdge(2,20), new HalfEdge(3,13),
            new HalfEdge(4,10), new HalfEdge(5,19), new HalfEdge(6,15), new HalfEdge(7,22),
            new HalfEdge(0,1), new HalfEdge(4,16), new HalfEdge(7,4), new HalfEdge(1,21),
            new HalfEdge(3,18), new HalfEdge(2,3), new HalfEdge(6,23), new HalfEdge(5,6),
            new HalfEdge(0,9), new HalfEdge(3,0), new HalfEdge(5,12), new HalfEdge(4,5),
            new HalfEdge(1,2), new HalfEdge(7,11), new HalfEdge(6,7), new HalfEdge(2,14),
        },
        new[]
        {
            new Polygon(0,4,0,surfaces[0],layers), new Polygon(4,4,1,surfaces[1],layers),
            new Polygon(8,4,2,surfaces[2],layers), new Polygon(12,4,3,surfaces[3],layers),
            new Polygon(16,4,4,surfaces[4],layers), new Polygon(20,4,5,surfaces[5],layers),
        });
}

record PerfCase(string Name, int Brushes, Func<int> Build);
record BrushMesh(Vec3[] Vertices, HalfEdge[] HalfEdges, Polygon[] Polygons);

enum Operation : byte { Additive = 0, Subtractive = 1, Intersecting = 2 }
enum BrushFlags : int { Default = 0, Infinite = 1 }
enum VertexChannelFlags : byte { Position = 0, Tangent = 2, Normal = 4, UV0 = 8, All = 14 }
enum LayerUsageFlags : int
{
    None = 0,
    Renderable = 1,
    CastShadows = 2,
    ReceiveShadows = 4,
    RenderCastShadows = Renderable | CastShadows,
    RenderReceiveShadows = Renderable | ReceiveShadows,
    RenderReceiveCastShadows = Renderable | CastShadows | ReceiveShadows,
    Collidable = 8,
    Culled = 1 << 23,
}
enum LayerParameterIndex : byte { None = 0, LayerParameter1 = 1, LayerParameter2 = 2, LayerParameter3 = 3 }

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct Vec2 { public float X, Y; }

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct Vec3(float x, float y, float z)
{
    public float X = x;
    public float Y = y;
    public float Z = z;
    public static Vec3 Zero => new(0, 0, 0);
    public static Vec3 Splat(float value) => new(value, value, value);
    public static Vec3 operator +(Vec3 a, Vec3 b) => new(a.X + b.X, a.Y + b.Y, a.Z + b.Z);
    public static Vec3 operator *(Vec3 a, float s) => new(a.X * s, a.Y * s, a.Z * s);
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct Vec4(float x, float y, float z, float w) { public float X = x, Y = y, Z = z, W = w; }

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct Mat4
{
    public float M00, M10, M20, M30;
    public float M01, M11, M21, M31;
    public float M02, M12, M22, M32;
    public float M03, M13, M23, M33;
    public static Mat4 Identity => new() { M00 = 1, M11 = 1, M22 = 1, M33 = 1 };
    public static Mat4 RotationZ(float radians)
    {
        var c = MathF.Cos(radians);
        var s = MathF.Sin(radians);
        return new Mat4 { M00 = c, M10 = s, M01 = -s, M11 = c, M22 = 1, M33 = 1 };
    }
    public Mat4 WithTranslation(Vec3 t)
    {
        var m = this;
        m.M03 = t.X; m.M13 = t.Y; m.M23 = t.Z; m.M33 = 1;
        return m;
    }
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct UvMatrix(Vec4 u, Vec4 v) { public Vec4 U = u; public Vec4 V = v; }

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct SurfaceDescription
{
    public uint SmoothingGroup;
    public int SurfaceFlags;
    public UvMatrix UV0;
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct SurfaceLayers
{
    public LayerUsageFlags LayerUsage;
    public int LayerParameter1;
    public int LayerParameter2;
    public int LayerParameter3;
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct Aabb
{
    public float MinX, MaxX;
    public float MinY, MaxY;
    public float MinZ, MaxZ;
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct Polygon(int firstEdge, int edgeCount, int polygonId, SurfaceDescription surface, SurfaceLayers layers)
{
    public int FirstEdge = firstEdge;
    public int EdgeCount = edgeCount;
    public int PolygonId = polygonId;
    public SurfaceDescription Surface = surface;
    public SurfaceLayers Layers = layers;
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct HalfEdge(int vertexIndex, int twinIndex)
{
    public int VertexIndex = vertexIndex;
    public int TwinIndex = twinIndex;
}

[StructLayout(LayoutKind.Sequential, Pack = 4)]
struct MeshQuery
{
    const int BitShift = 24;
    public uint Layers;
    public uint MaskAndChannels;
    public static MeshQuery RenderableAll => new(LayerUsageFlags.Renderable, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All);
    public static MeshQuery[] SimpleTypes => new[]
    {
        new MeshQuery(LayerUsageFlags.Renderable, LayerUsageFlags.Renderable, LayerParameterIndex.None, VertexChannelFlags.Position),
        new MeshQuery(LayerUsageFlags.RenderReceiveCastShadows, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.None, VertexChannelFlags.Position),
        new MeshQuery(LayerUsageFlags.Renderable, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.None, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.RenderReceiveCastShadows, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.None, LayerUsageFlags.None, LayerParameterIndex.None, VertexChannelFlags.Position),
        new MeshQuery(LayerUsageFlags.Culled, LayerUsageFlags.Culled, LayerParameterIndex.None, VertexChannelFlags.Position),
    };
    public static MeshQuery[] RenderOnlyTypes => new[]
    {
        new MeshQuery(LayerUsageFlags.CastShadows, LayerUsageFlags.RenderCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.Renderable, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.RenderCastShadows, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.RenderReceiveShadows, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.RenderReceiveCastShadows, LayerUsageFlags.RenderReceiveCastShadows, LayerParameterIndex.LayerParameter1, VertexChannelFlags.All),
        new MeshQuery(LayerUsageFlags.None, LayerUsageFlags.Renderable, LayerParameterIndex.None, VertexChannelFlags.Position),
        new MeshQuery(LayerUsageFlags.CastShadows, LayerUsageFlags.None, LayerParameterIndex.None, VertexChannelFlags.Position),
        new MeshQuery(LayerUsageFlags.ReceiveShadows, LayerUsageFlags.None, LayerParameterIndex.None, VertexChannelFlags.Position),
        new MeshQuery(LayerUsageFlags.Culled, LayerUsageFlags.None, LayerParameterIndex.None, VertexChannelFlags.Position),
    };

    public MeshQuery(LayerUsageFlags query, LayerUsageFlags mask, LayerParameterIndex parameterIndex, VertexChannelFlags vertexChannels)
    {
        if (mask == LayerUsageFlags.None)
            mask = query;
        Layers = (uint)query;
        Layers |= (uint)parameterIndex << BitShift;
        MaskAndChannels = (uint)mask | ((uint)vertexChannels << BitShift);
    }
    public int UsedVertexChannels => (int)(MaskAndChannels >> BitShift);
}

[StructLayout(LayoutKind.Sequential, Pack = 8)]
struct GeneratedMeshDescription
{
    public MeshQuery MeshQuery;
    public int SurfaceParameter;
    public int MeshQueryIndex;
    public int SubMeshQueryIndex;
    public long GeometryHashValue;
    public long SurfaceHashValue;
    public int VertexCount;
    public int IndexCount;
}

static partial class Native
{
    const string Dll = "RealtimeCSG[1_559]";
    static readonly StringLog Log = (message, userId) => Console.Error.WriteLine($"native[{userId}]: {message}");
    static readonly ReturnString NameForUserId = _ => "<native-bridge>";

    public static void RegisterNoopUnityMethods()
    {
        var methods = new UnityMethods
        {
            DebugLog = Log,
            DebugLogError = Log,
            DebugLogWarning = Log,
            NameForUserID = NameForUserId,
        };
        RegisterMethods(ref methods);
    }

    public delegate void StringLog([MarshalAs(UnmanagedType.LPStr)] string text, int uniqueObjectID);
    [return: MarshalAs(UnmanagedType.LPStr)]
    public delegate string ReturnString(int uniqueObjectID);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi, Pack = 4)]
    struct UnityMethods
    {
        public StringLog DebugLog;
        public StringLog DebugLogError;
        public StringLog DebugLogWarning;
        public ReturnString NameForUserID;
    }

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] static extern void RegisterMethods(ref UnityMethods unityMethods);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern void ClearAllNodes();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool UpdateAllTreeMeshes();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern void RebuildAll();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetNodeCount();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetBrushCount();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetTreeCount();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetBrushMeshCount();
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GenerateTree(int userID, out int generatedTreeNodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetNumberOfBrushesInTree(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetDirty(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool ClearDirty(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool IsNodeDirty(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetChildNodeCount(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetChildNodeAtIndex(int nodeID, int index);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetTreeOfNode(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern byte GetTypeOfNode(int nodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetTreeEnabled(int modelNodeID, bool isEnabled);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GetTreeEnabled(int modelNodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GenerateBrush(int userID, out int generatedNodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GenerateBranch(int userID, out int generatedOperationNodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetBranchOperationType(int branchNodeID, Operation operation);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetBrushOperationType(int brushNodeID, Operation operation);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetBrushFlags(int brushNodeID, BrushFlags flags);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetBrushOperationType(int brushNodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetBrushMeshID(int brushNodeID, int brushMeshID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int GetBrushMeshID(int brushNodeID);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GetBrushBounds(int brushNodeID, ref Aabb bounds);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GetBrushOutlineSizes(int brushNodeID, out int vertexCount, out int visibleOuterLineCount, out int visibleInnerLineCount, out int invisibleOuterLineCount, out int invisibleInnerLineCount, out int invalidLineCount);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetNodeLocalTransformation(int brushNodeID, ref Mat4 brushToTreeSpace);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool SetChildNodes(int nodeID, int childCount, IntPtr childrenNodeIDs);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool InsertChildNodeRange(int nodeID, int index, int childCount, IntPtr childrenNodeIDs);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int CreateBrushMesh(int userID, int vertexCount, IntPtr vertices, int halfEdgeCount, IntPtr halfEdges, int polygonCount, IntPtr polygons);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool UpdateBrushMesh(int brushMeshIndex, int vertexCount, IntPtr vertices, int halfEdgeCount, IntPtr halfEdges, int polygonCount, IntPtr polygons);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool IsBrushMeshIDValid(int brushMeshIndex);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GenerateMeshDescriptions(int treeNodeID, int meshTypeCount, IntPtr meshTypes, VertexChannelFlags vertexChannelMask, out int meshDescriptionCount);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GetMeshDescriptions(int treeNodeID, int meshDescriptionCount, IntPtr meshDescriptions);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern bool GetGeneratedMesh(int treeNodeID, int meshIndex, int subMeshIndex, int indexCount, IntPtr indices, int vertexCount, IntPtr positions, IntPtr tangents, IntPtr normals, IntPtr uvs, out Vec3 boundsCenter, out Vec3 boundsSize);
    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)] public static extern int RayCastIntoTreeMultiCount(int modelNodeID, ref Vec3 worldRayStart, ref Vec3 worldRayEnd, ref Mat4 modelLocalToWorldMatrix, int inFilterLayerParameter0, bool ignoreInvisiblePolygons, IntPtr ignoreNodeIds, int ignoreNodeIdCount);
}
