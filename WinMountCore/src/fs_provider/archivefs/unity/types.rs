use std::{
    collections::{BTreeMap, HashMap},
    io::{BufRead, Cursor, Read, Seek, Write},
};

use binrw::{binread, BinRead, NamedArgs, NullString};
use modular_bitfield::prelude::*;

// See https://github.com/Perfare/AssetStudio/tree/master/AssetStudio/ClassIDType.cs
#[derive(num_derive::FromPrimitive, strum_macros::Display)]
pub enum UnityObjectClassIDType {
    UnknownType = -1,
    Object = 0,
    GameObject = 1,
    Component = 2,
    LevelGameManager = 3,
    Transform = 4,
    TimeManager = 5,
    GlobalGameManager = 6,
    Behaviour = 8,
    GameManager = 9,
    AudioManager = 11,
    ParticleAnimator = 12,
    InputManager = 13,
    EllipsoidParticleEmitter = 15,
    Pipeline = 17,
    EditorExtension = 18,
    Physics2DSettings = 19,
    Camera = 20,
    Material = 21,
    MeshRenderer = 23,
    Renderer = 25,
    ParticleRenderer = 26,
    Texture = 27,
    Texture2D = 28,
    OcclusionCullingSettings = 29,
    GraphicsSettings = 30,
    MeshFilter = 33,
    OcclusionPortal = 41,
    Mesh = 43,
    Skybox = 45,
    QualitySettings = 47,
    Shader = 48,
    TextAsset = 49,
    Rigidbody2D = 50,
    Physics2DManager = 51,
    Collider2D = 53,
    Rigidbody = 54,
    PhysicsManager = 55,
    Collider = 56,
    Joint = 57,
    CircleCollider2D = 58,
    HingeJoint = 59,
    PolygonCollider2D = 60,
    BoxCollider2D = 61,
    PhysicsMaterial2D = 62,
    MeshCollider = 64,
    BoxCollider = 65,
    CompositeCollider2D = 66,
    EdgeCollider2D = 68,
    CapsuleCollider2D = 70,
    ComputeShader = 72,
    AnimationClip = 74,
    ConstantForce = 75,
    WorldParticleCollider = 76,
    TagManager = 78,
    AudioListener = 81,
    AudioSource = 82,
    AudioClip = 83,
    RenderTexture = 84,
    CustomRenderTexture = 86,
    MeshParticleEmitter = 87,
    ParticleEmitter = 88,
    Cubemap = 89,
    Avatar = 90,
    AnimatorController = 91,
    GUILayer = 92,
    RuntimeAnimatorController = 93,
    ScriptMapper = 94,
    Animator = 95,
    TrailRenderer = 96,
    DelayedCallManager = 98,
    TextMesh = 102,
    RenderSettings = 104,
    Light = 108,
    CGProgram = 109,
    BaseAnimationTrack = 110,
    Animation = 111,
    MonoBehaviour = 114,
    MonoScript = 115,
    MonoManager = 116,
    Texture3D = 117,
    NewAnimationTrack = 118,
    Projector = 119,
    LineRenderer = 120,
    Flare = 121,
    Halo = 122,
    LensFlare = 123,
    FlareLayer = 124,
    HaloLayer = 125,
    NavMeshAreas = 126,
    // NavMeshProjectSettings = 126,
    HaloManager = 127,
    Font = 128,
    PlayerSettings = 129,
    NamedObject = 130,
    GUITexture = 131,
    GUIText = 132,
    GUIElement = 133,
    PhysicMaterial = 134,
    SphereCollider = 135,
    CapsuleCollider = 136,
    SkinnedMeshRenderer = 137,
    FixedJoint = 138,
    RaycastCollider = 140,
    BuildSettings = 141,
    AssetBundle = 142,
    CharacterController = 143,
    CharacterJoint = 144,
    SpringJoint = 145,
    WheelCollider = 146,
    ResourceManager = 147,
    NetworkView = 148,
    NetworkManager = 149,
    PreloadData = 150,
    MovieTexture = 152,
    ConfigurableJoint = 153,
    TerrainCollider = 154,
    MasterServerInterface = 155,
    TerrainData = 156,
    LightmapSettings = 157,
    WebCamTexture = 158,
    EditorSettings = 159,
    InteractiveCloth = 160,
    ClothRenderer = 161,
    EditorUserSettings = 162,
    SkinnedCloth = 163,
    AudioReverbFilter = 164,
    AudioHighPassFilter = 165,
    AudioChorusFilter = 166,
    AudioReverbZone = 167,
    AudioEchoFilter = 168,
    AudioLowPassFilter = 169,
    AudioDistortionFilter = 170,
    SparseTexture = 171,
    AudioBehaviour = 180,
    AudioFilter = 181,
    WindZone = 182,
    Cloth = 183,
    SubstanceArchive = 184,
    ProceduralMaterial = 185,
    ProceduralTexture = 186,
    Texture2DArray = 187,
    CubemapArray = 188,
    OffMeshLink = 191,
    OcclusionArea = 192,
    Tree = 193,
    NavMeshObsolete = 194,
    NavMeshAgent = 195,
    NavMeshSettings = 196,
    LightProbesLegacy = 197,
    ParticleSystem = 198,
    ParticleSystemRenderer = 199,
    ShaderVariantCollection = 200,
    LODGroup = 205,
    BlendTree = 206,
    Motion = 207,
    NavMeshObstacle = 208,
    SortingGroup = 210,
    SpriteRenderer = 212,
    Sprite = 213,
    CachedSpriteAtlas = 214,
    ReflectionProbe = 215,
    ReflectionProbes = 216,
    Terrain = 218,
    LightProbeGroup = 220,
    AnimatorOverrideController = 221,
    CanvasRenderer = 222,
    Canvas = 223,
    RectTransform = 224,
    CanvasGroup = 225,
    BillboardAsset = 226,
    BillboardRenderer = 227,
    SpeedTreeWindAsset = 228,
    AnchoredJoint2D = 229,
    Joint2D = 230,
    SpringJoint2D = 231,
    DistanceJoint2D = 232,
    HingeJoint2D = 233,
    SliderJoint2D = 234,
    WheelJoint2D = 235,
    ClusterInputManager = 236,
    BaseVideoTexture = 237,
    NavMeshData = 238,
    AudioMixer = 240,
    AudioMixerController = 241,
    AudioMixerGroupController = 243,
    AudioMixerEffectController = 244,
    AudioMixerSnapshotController = 245,
    PhysicsUpdateBehaviour2D = 246,
    ConstantForce2D = 247,
    Effector2D = 248,
    AreaEffector2D = 249,
    PointEffector2D = 250,
    PlatformEffector2D = 251,
    SurfaceEffector2D = 252,
    BuoyancyEffector2D = 253,
    RelativeJoint2D = 254,
    FixedJoint2D = 255,
    FrictionJoint2D = 256,
    TargetJoint2D = 257,
    LightProbes = 258,
    LightProbeProxyVolume = 259,
    SampleClip = 271,
    AudioMixerSnapshot = 272,
    AudioMixerGroup = 273,
    NScreenBridge = 280,
    AssetBundleManifest = 290,
    UnityAdsManager = 292,
    RuntimeInitializeOnLoadManager = 300,
    CloudWebServicesManager = 301,
    UnityAnalyticsManager = 303,
    CrashReportManager = 304,
    PerformanceReportingManager = 305,
    UnityConnectSettings = 310,
    AvatarMask = 319,
    PlayableDirector = 320,
    VideoPlayer = 328,
    VideoClip = 329,
    ParticleSystemForceField = 330,
    SpriteMask = 331,
    WorldAnchor = 362,
    OcclusionCullingData = 363,
    //kLargestRuntimeClassID = 364
    SmallestEditorClassID = 1000,
    PrefabInstance = 1001,
    EditorExtensionImpl = 1002,
    AssetImporter = 1003,
    AssetDatabaseV1 = 1004,
    Mesh3DSImporter = 1005,
    TextureImporter = 1006,
    ShaderImporter = 1007,
    ComputeShaderImporter = 1008,
    AudioImporter = 1020,
    HierarchyState = 1026,
    GUIDSerializer = 1027,
    AssetMetaData = 1028,
    DefaultAsset = 1029,
    DefaultImporter = 1030,
    TextScriptImporter = 1031,
    SceneAsset = 1032,
    NativeFormatImporter = 1034,
    MonoImporter = 1035,
    AssetServerCache = 1037,
    LibraryAssetImporter = 1038,
    ModelImporter = 1040,
    FBXImporter = 1041,
    TrueTypeFontImporter = 1042,
    MovieImporter = 1044,
    EditorBuildSettings = 1045,
    DDSImporter = 1046,
    InspectorExpandedState = 1048,
    AnnotationManager = 1049,
    PluginImporter = 1050,
    EditorUserBuildSettings = 1051,
    PVRImporter = 1052,
    ASTCImporter = 1053,
    KTXImporter = 1054,
    IHVImageFormatImporter = 1055,
    AnimatorStateTransition = 1101,
    AnimatorState = 1102,
    HumanTemplate = 1105,
    AnimatorStateMachine = 1107,
    PreviewAnimationClip = 1108,
    AnimatorTransition = 1109,
    SpeedTreeImporter = 1110,
    AnimatorTransitionBase = 1111,
    SubstanceImporter = 1112,
    LightmapParameters = 1113,
    LightingDataAsset = 1120,
    GISRaster = 1121,
    GISRasterImporter = 1122,
    CadImporter = 1123,
    SketchUpImporter = 1124,
    BuildReport = 1125,
    PackedAssets = 1126,
    VideoClipImporter = 1127,
    ActivationLogComponent = 2000,
    //kLargestEditorClassID = 2001
    //kClassIdOutOfHierarchy = 100000
    //int = 100000,
    //bool = 100001,
    //float = 100002,
    MonoObject = 100003,
    Collision = 100004,
    Vector3f = 100005,
    RootMotionData = 100006,
    Collision2D = 100007,
    AudioMixerLiveUpdateFloat = 100008,
    AudioMixerLiveUpdateBool = 100009,
    Polygon2D = 100010,
    //void = 100011,
    TilemapCollider2D = 19719996,
    AssetImporterLog = 41386430,
    VFXRenderer = 73398921,
    SerializableManagedRefTestClass = 76251197,
    Grid = 156049354,
    ScenesUsingAssets = 156483287,
    ArticulationBody = 171741748,
    Preset = 181963792,
    EmptyObject = 277625683,
    IConstraint = 285090594,
    TestObjectWithSpecialLayoutOne = 293259124,
    AssemblyDefinitionReferenceImporter = 294290339,
    SiblingDerived = 334799969,
    TestObjectWithSerializedMapStringNonAlignedStruct = 342846651,
    SubDerived = 367388927,
    AssetImportInProgressProxy = 369655926,
    PluginBuildInfo = 382020655,
    EditorProjectAccess = 426301858,
    PrefabImporter = 468431735,
    TestObjectWithSerializedArray = 478637458,
    TestObjectWithSerializedAnimationCurve = 478637459,
    TilemapRenderer = 483693784,
    ScriptableCamera = 488575907,
    SpriteAtlasAsset = 612988286,
    SpriteAtlasDatabase = 638013454,
    AudioBuildInfo = 641289076,
    CachedSpriteAtlasRuntimeData = 644342135,
    RendererFake = 646504946,
    AssemblyDefinitionReferenceAsset = 662584278,
    BuiltAssetBundleInfoSet = 668709126,
    SpriteAtlas = 687078895,
    RayTracingShaderImporter = 747330370,
    RayTracingShader = 825902497,
    LightingSettings = 850595691,
    PlatformModuleSetup = 877146078,
    VersionControlSettings = 890905787,
    AimConstraint = 895512359,
    VFXManager = 937362698,
    VisualEffectSubgraph = 994735392,
    VisualEffectSubgraphOperator = 994735403,
    VisualEffectSubgraphBlock = 994735404,
    LocalizationImporter = 1027052791,
    Derived = 1091556383,
    PropertyModificationsTargetTestObject = 1111377672,
    ReferencesArtifactGenerator = 1114811875,
    AssemblyDefinitionAsset = 1152215463,
    SceneVisibilityState = 1154873562,
    LookAtConstraint = 1183024399,
    SpriteAtlasImporter = 1210832254,
    MultiArtifactTestImporter = 1223240404,
    GameObjectRecorder = 1268269756,
    LightingDataAssetParent = 1325145578,
    PresetManager = 1386491679,
    TestObjectWithSpecialLayoutTwo = 1392443030,
    StreamingManager = 1403656975,
    LowerResBlitTexture = 1480428607,
    StreamingController = 1542919678,
    RenderPassAttachment = 1571458007,
    TestObjectVectorPairStringBool = 1628831178,
    GridLayout = 1742807556,
    AssemblyDefinitionImporter = 1766753193,
    ParentConstraint = 1773428102,
    FakeComponent = 1803986026,
    PositionConstraint = 1818360608,
    RotationConstraint = 1818360609,
    ScaleConstraint = 1818360610,
    Tilemap = 1839735485,
    PackageManifest = 1896753125,
    PackageManifestImporter = 1896753126,
    TerrainLayer = 1953259897,
    SpriteShapeRenderer = 1971053207,
    NativeObjectType = 1977754360,
    TestObjectWithSerializedMapStringBool = 1981279845,
    SerializableManagedHost = 1995898324,
    VisualEffectAsset = 2058629509,
    VisualEffectImporter = 2058629510,
    VisualEffectResource = 2058629511,
    VisualEffectObject = 2059678085,
    VisualEffect = 2083052967,
    LocalizationAsset = 2083778819,
    ScriptedImporter = 2089858483,
}

#[derive(num_derive::FromPrimitive, strum_macros::Display)]
pub enum UnityBuildTarget {
    NoTarget = -2,
    AnyPlayer = -1,
    ValidPlayer = 1,
    StandaloneOSX = 2,
    StandaloneOSXPPC = 3,
    StandaloneOSXIntel = 4,
    StandaloneWindows,
    WebPlayer,
    WebPlayerStreamed,
    Wii = 8,
    iOS = 9,
    PS3,
    XBOX360,
    Broadcom = 12,
    Android = 13,
    StandaloneGLESEmu = 14,
    StandaloneGLES20Emu = 15,
    NaCl = 16,
    StandaloneLinux = 17,
    FlashPlayer = 18,
    StandaloneWindows64 = 19,
    WebGL,
    WSAPlayer,
    StandaloneLinux64 = 24,
    StandaloneLinuxUniversal,
    WP8Player,
    StandaloneOSXIntel64,
    BlackBerry,
    Tizen,
    PSP2,
    PS4,
    PSM,
    XboxOne,
    SamsungTV,
    N3DS,
    WiiU,
    tvOS,
    Switch,
    Lumin,
    Stadia,
    CloudRendering,
    GameCoreXboxSeries,
    GameCoreXboxOne,
    PS5,
    EmbeddedLinux,
    QNX,
    UnknownPlatform = 9999,
}

#[derive(num_derive::FromPrimitive, strum_macros::Display)]
pub enum UnityTextureFormat {
    Alpha8 = 1,
    ARGB4444,
    RGB24,
    RGBA32,
    ARGB32,
    ARGBFloat,
    RGB565,
    BGR24,
    R16,
    DXT1,
    DXT3,
    DXT5,
    RGBA4444,
    BGRA32,
    RHalf,
    RGHalf,
    RGBAHalf,
    RFloat,
    RGFloat,
    RGBAFloat,
    YUY2,
    RGB9e5Float,
    RGBFloat,
    BC6H,
    BC7,
    BC4,
    BC5,
    DXT1Crunched,
    DXT5Crunched,
    PVRTC_RGB2,
    PVRTC_RGBA2,
    PVRTC_RGB4,
    PVRTC_RGBA4,
    ETC_RGB4,
    ATC_RGB4,
    ATC_RGBA8,
    EAC_R = 41,
    EAC_R_SIGNED,
    EAC_RG,
    EAC_RG_SIGNED,
    ETC2_RGB,
    ETC2_RGBA1,
    ETC2_RGBA8,
    ASTC_RGB_4x4,
    ASTC_RGB_5x5,
    ASTC_RGB_6x6,
    ASTC_RGB_8x8,
    ASTC_RGB_10x10,
    ASTC_RGB_12x12,
    ASTC_RGBA_4x4,
    ASTC_RGBA_5x5,
    ASTC_RGBA_6x6,
    ASTC_RGBA_8x8,
    ASTC_RGBA_10x10,
    ASTC_RGBA_12x12,
    ETC_RGB4_3DS,
    ETC_RGBA8_3DS,
    RG16,
    R8,
    ETC_RGB4Crunched,
    ETC2_RGBA8Crunched,
    ASTC_HDR_4x4,
    ASTC_HDR_5x5,
    ASTC_HDR_6x6,
    ASTC_HDR_8x8,
    ASTC_HDR_10x10,
    ASTC_HDR_12x12,
    RG32,
    RGB48,
    RGBA64,
}

fn parse_unity_engine_version(input: &str) -> nom::IResult<&str, UnityEngineVersion> {
    use nom::{
        bytes::complete::tag, character::complete::digit1, combinator::map_res, sequence::tuple,
    };
    let parse_u32 = |input| map_res(digit1, |s: &str| s.parse::<u32>())(input);
    let dot = |input| tag(".")(input);
    let (input, (v0, _, v1, _, v2)) = tuple((parse_u32, dot, parse_u32, dot, parse_u32))(input)?;
    Ok((input, UnityEngineVersion(v0, v1, v2)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnityEngineVersion(u32, u32, u32);
impl TryFrom<&str> for UnityEngineVersion {
    type Error = nom::error::Error<String>;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        use nom::Finish;
        Ok(parse_unity_engine_version(value)
            .map_err(|e| e.to_owned())
            .finish()?
            .1)
    }
}

#[derive(BitfieldSpecifier, Debug)]
#[bits = 6]
pub enum UnityFSAssetCompressionType {
    None,
    Lzma,
    Lz4,
    Lz4HC,
    Lzham,
}

#[bitfield]
#[derive(BinRead, Debug)]
#[br(big, map = Self::from_be_bytes)]
pub struct UnityFSAssetBundleFlags {
    pub compression_type: UnityFSAssetCompressionType,
    pub has_dir_info: bool,
    pub block_and_dir_info_at_eof: bool,
    #[skip]
    __: B1,
    pub has_padding_before_blocks: bool,
    #[skip]
    __: B22,
}
impl UnityFSAssetBundleFlags {
    fn from_be_bytes(mut bytes: [u8; 4]) -> Self {
        bytes.reverse();
        Self::from_bytes(bytes)
    }
}

#[derive(BinRead, Debug)]
#[br(big, magic = b"UnityFS\0")]
pub struct UnityFSAssetBundle {
    pub file_ver: u32,
    pub min_player_ver: NullString,
    pub file_engine_ver: NullString,
    pub total_file_size: u64,
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub flags: UnityFSAssetBundleFlags,
}

#[binread]
#[derive(Debug)]
#[br(big)]
pub struct UnityFSBlocksAndDirsInfo {
    pub uncompressed_data_hash: [u8; 16],
    #[br(temp)]
    blocks_info_count: u32,
    #[br(count = blocks_info_count)]
    pub blocks_info: Vec<UnityFSBlockInfo>,
    #[br(temp)]
    dirs_info_count: u32,
    #[br(count = dirs_info_count)]
    pub dirs_info: Vec<UnityFSDirInfo>,
}

#[bitfield]
#[derive(BinRead, Debug)]
#[br(big, map = Self::from_be_bytes)]
pub struct UnityFSAssetBlockFlags {
    pub compression_type: UnityFSAssetCompressionType,
    #[skip]
    __: B10,
}
impl UnityFSAssetBlockFlags {
    fn from_be_bytes(mut bytes: [u8; 2]) -> Self {
        bytes.reverse();
        Self::from_bytes(bytes)
    }
}

#[derive(BinRead, Debug)]
pub struct UnityFSBlockInfo {
    pub uncompressed_size: u32,
    pub compressed_size: u32,
    pub flags: UnityFSAssetBlockFlags,
}

#[derive(BinRead, Debug)]
pub struct UnityFSDirInfo {
    pub offset: u64,
    pub size: u64,
    pub flags: u32,
    pub path: NullString,
}

#[binread]
#[derive(Debug)]
#[br(big)]
pub struct UnityFSSerializedFileInfo {
    #[br(temp)]
    legacy_metadata_size: u32,
    #[br(temp)]
    legacy_file_size: u32,
    // See https://github.com/Perfare/AssetStudio/tree/master/AssetStudio/SerializedFileFormatVersion.cs#L9
    #[br(assert(version >= 9, "file version too low (expected >=9, found {version})"))]
    pub version: u32,
    #[br(temp)]
    legacy_data_offset: u32,
    pub endianness: u8,
    #[br(temp)]
    reserved1: [u8; 3],
    #[br(if(version >= 22, legacy_metadata_size))]
    pub metadata_size: u32,
    #[br(if(version >= 22, legacy_file_size as _))]
    pub file_size: u64,
    #[br(if(version >= 22, legacy_data_offset as _))]
    pub data_offset: u64,
    #[br(temp)]
    reserved2: u64,
    #[br(assert(unity_version.len() <= 20))]
    pub unity_version: NullString,
    #[br(is_little = (endianness == 0))]
    pub target_platform: u32,
    #[br(if(version >= 13), map = |x: u8| x != 0)]
    pub enable_type_tree: bool,
    #[br(is_little = (endianness == 0), temp)]
    types_count: u32,
    #[br(count = types_count, is_little = (endianness == 0), args { inner: binrw::args! { version, is_ref: false, enable_type_tree } })]
    pub types: Vec<UnityFSSerializedTypeInfo>,
    #[br(if(version >= 7 && version < 14), map = |x: u32| x != 0)]
    pub big_id_enabled: bool,
    #[br(is_little = (endianness == 0), temp)]
    objects_count: u32,
    #[br(count = objects_count, is_little = (endianness == 0), args { inner: binrw::args! { version, big_id_enabled, types: &types } })]
    pub objects: Vec<UnityFSSerializedObjectInfo>,
    #[br(if(version >= 11), is_little = (endianness == 0), temp)]
    scripts_count: u32,
    #[br(if(version >= 11), count = scripts_count, is_little = (endianness == 0), args { inner: binrw::args! { version } })]
    pub scripts: Vec<UnityFSSerializedObjectIdentifierInfo>,
    #[br(is_little = (endianness == 0), temp)]
    externals_count: u32,
    #[br(count = externals_count, is_little = (endianness == 0), args { inner: binrw::args! { version } })]
    pub externals: Vec<UnityFSSerializedFileIdentifierInfo>,
    #[br(if(version >= 20), is_little = (endianness == 0), temp)]
    ref_types_count: u32,
    #[br(count = ref_types_count, is_little = (endianness == 0), args { inner: binrw::args! { version, is_ref: true, enable_type_tree } })]
    pub ref_types: Vec<UnityFSSerializedTypeInfo>,
    pub user_information: NullString,
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32 })]
pub struct UnityFSSerializedFileIdentifierInfo {
    #[br(temp)]
    reserved1: NullString,
    pub guid: [u8; 16],
    pub type_id: u32,
    pub path_name: NullString,
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32 })]
pub struct UnityFSSerializedObjectIdentifierInfo {
    pub local_file_index: u32,
    #[br(if(version < 14), temp)]
    legacy_identifier_in_file: u32,
    #[br(if(version >= 14, legacy_identifier_in_file as _), align_before = 4)]
    pub identifier_in_file: u64,
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32, big_id_enabled: bool, types: &[UnityFSSerializedTypeInfo] })]
pub struct UnityFSSerializedObjectInfo {
    #[br(if(!big_id_enabled && version < 14), temp)]
    legacy_path_id: u32,
    #[br(if(!big_id_enabled && version >= 14), align_before = 4, temp)]
    legacy2_path_id: u64,
    #[br(if(big_id_enabled, legacy_path_id as u64 + legacy2_path_id))]
    pub path_id: u64,
    #[br(if(version < 22), temp)]
    legacy_byte_start: u32,
    #[br(if(version >= 22, legacy_byte_start as _))]
    pub byte_start: u64,
    pub byte_size: u32,
    pub type_id: u32,
    #[br(if(version < 16), temp)]
    legacy_class_id: u16,
    #[br(if(version >= 16, legacy_class_id as _), calc = types[type_id as usize].class_id as _)]
    pub class_id: u32,
    #[br(if(version < 11), map = |x: u16| x != 0)]
    pub is_destroyed: bool,
    #[br(if(version >= 11 && version < 17))]
    pub script_type_index: u16,
    #[br(if(version == 15 || version == 16), map = |x: u8| x != 0)]
    pub stripped: bool,
}

fn get_predefined_str(index: u32) -> Option<&'static str> {
    Some(match index {
        0 => "AABB",
        5 => "AnimationClip",
        19 => "AnimationCurve",
        34 => "AnimationState",
        49 => "Array",
        55 => "Base",
        60 => "BitField",
        69 => "bitset",
        76 => "bool",
        81 => "char",
        86 => "ColorRGBA",
        96 => "Component",
        106 => "data",
        111 => "deque",
        117 => "double",
        124 => "dynamic_array",
        138 => "FastPropertyName",
        155 => "first",
        161 => "float",
        167 => "Font",
        172 => "GameObject",
        183 => "Generic Mono",
        196 => "GradientNEW",
        208 => "GUID",
        213 => "GUIStyle",
        222 => "int",
        226 => "list",
        231 => "long long",
        241 => "map",
        245 => "Matrix4x4f",
        256 => "MdFour",
        263 => "MonoBehaviour",
        277 => "MonoScript",
        288 => "m_ByteSize",
        299 => "m_Curve",
        307 => "m_EditorClassIdentifier",
        331 => "m_EditorHideFlags",
        349 => "m_Enabled",
        359 => "m_ExtensionPtr",
        374 => "m_GameObject",
        387 => "m_Index",
        395 => "m_IsArray",
        405 => "m_IsStatic",
        416 => "m_MetaFlag",
        427 => "m_Name",
        434 => "m_ObjectHideFlags",
        452 => "m_PrefabInternal",
        469 => "m_PrefabParentObject",
        490 => "m_Script",
        499 => "m_StaticEditorFlags",
        519 => "m_Type",
        526 => "m_Version",
        536 => "Object",
        543 => "pair",
        548 => "PPtr<Component>",
        564 => "PPtr<GameObject>",
        581 => "PPtr<Material>",
        596 => "PPtr<MonoBehaviour>",
        616 => "PPtr<MonoScript>",
        633 => "PPtr<Object>",
        646 => "PPtr<Prefab>",
        659 => "PPtr<Sprite>",
        672 => "PPtr<TextAsset>",
        688 => "PPtr<Texture>",
        702 => "PPtr<Texture2D>",
        718 => "PPtr<Transform>",
        734 => "Prefab",
        741 => "Quaternionf",
        753 => "Rectf",
        759 => "RectInt",
        767 => "RectOffset",
        778 => "second",
        785 => "set",
        789 => "short",
        795 => "size",
        800 => "SInt16",
        807 => "SInt32",
        814 => "SInt64",
        821 => "SInt8",
        827 => "staticvector",
        840 => "string",
        847 => "TextAsset",
        857 => "TextMesh",
        866 => "Texture",
        874 => "Texture2D",
        884 => "Transform",
        894 => "TypelessData",
        907 => "UInt16",
        914 => "UInt32",
        921 => "UInt64",
        928 => "UInt8",
        934 => "unsigned int",
        947 => "unsigned long long",
        966 => "unsigned short",
        981 => "vector",
        988 => "Vector2f",
        997 => "Vector3f",
        1006 => "Vector4f",
        1015 => "m_ScriptingClassIdentifier",
        1042 => "Gradient",
        1051 => "Type*",
        1057 => "int2_storage",
        1070 => "int3_storage",
        1083 => "BoundsInt",
        1093 => "m_CorrespondingSourceObject",
        1121 => "m_PrefabInstance",
        1138 => "m_PrefabAsset",
        1152 => "FileSize",
        1161 => "Hash128",
        _ => return None,
    })
}

// #[derive(BinRead, Debug)]
// #[br(import { version: u32 })]
// pub struct UnityFSSerializedTypeTreeInfo {
//     nodes_count: u32,
//     str_buf_size: u32,
//     #[br(count = nodes_count, args { inner: binrw::args! { version } })]
//     nodes: Vec<UnityFSSerializedTypeTreeNodeInfo>,
//     #[br(count = str_buf_size)]
//     str_buf: Vec<u8>,
// }
#[derive(Debug)]
pub struct UnityFSSerializedTypeTreeInfo {
    pub nodes_count: u32,
    pub nodes: Vec<UnityFSSerializedTypeTreeNodeInfo>,
}
#[derive(NamedArgs, Clone)]
pub struct UnityFSSerializedTypeTreeInfoArgs {
    pub version: u32,
}
impl BinRead for UnityFSSerializedTypeTreeInfo {
    type Args<'a> = UnityFSSerializedTypeTreeInfoArgs;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let nodes_count = <_>::read_options(reader, endian, ())?;
        let str_buf_size = u32::read_options(reader, endian, ())?;
        let mut nodes = <Vec<UnityFSSerializedTypeTreeNodeInfo>>::read_options(
            reader,
            endian,
            binrw::args! { count: nodes_count as _, inner: binrw::args! { version: args.version } },
        )?;
        let str_buf =
            <Vec<u8>>::read_options(reader, endian, binrw::args! { count: str_buf_size as _ })?;
        let mut str_buf = Cursor::new(str_buf);

        // Load names
        let mut load_str_fn = |x: u32| -> binrw::BinResult<NullString> {
            if (x & 0x80000000) == 0 {
                // Offset
                str_buf.set_position(x as _);
                NullString::read(&mut str_buf)
            } else {
                // Common string
                Ok(match get_predefined_str(x & 0x7fffffff) {
                    Some(s) => s.into(),
                    None => x.to_string().into(),
                })
            }
        };
        for i in nodes.iter_mut() {
            i.type_str = load_str_fn(i.type_str_offset)?;
            i.name_str = load_str_fn(i.name_str_offset)?;
        }

        Ok(Self { nodes_count, nodes })
    }
}

#[derive(BinRead, Debug)]
#[br(import { version: u32 })]
pub struct UnityFSSerializedTypeTreeNodeInfo {
    pub node_version: u16,
    pub level: u8,
    pub type_flags: u8,
    pub type_str_offset: u32,
    pub name_str_offset: u32,
    pub byte_size: u32,
    pub index: u32,
    pub meta_flag: u32,
    #[br(if(version >= 19))]
    pub ref_type_hash: u64,
    #[br(ignore)]
    pub type_str: NullString,
    #[br(ignore)]
    pub name_str: NullString,
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32, is_ref: bool, enable_type_tree: bool })]
pub struct UnityFSSerializedTypeInfo {
    pub class_id: i32,
    #[br(if(version >= 16), map = |x: u8| x != 0)]
    pub is_stripped_type: bool,
    #[br(if(version >= 17))]
    pub script_type_index: u16,
    #[br(if(version >= 13 && (
        (is_ref && script_type_index > 0) ||
        ((version < 16 && class_id < 0) || (version >= 16 && class_id == 114))
    )))]
    pub script_id: [u8; 16],
    #[br(if(version >= 13))]
    pub old_type_hash: [u8; 16],
    // TODO: Properly handle type tree
    #[br(args { version })]
    pub type_tree: UnityFSSerializedTypeTreeInfo,
    #[br(if(version >= 21 && is_ref))]
    pub class_name: NullString,
    #[br(if(version >= 21 && is_ref))]
    pub namespace: NullString,
    #[br(if(version >= 21 && is_ref))]
    pub asm_name: NullString,
    #[br(if(version >= 21 && !is_ref), temp)]
    type_dependencies_count: u32,
    #[br(if(version >= 21 && !is_ref), count = type_dependencies_count)]
    pub type_dependencies: Vec<u32>,
}

#[binread]
#[derive(Debug, Default)]
#[br(import { version: u32 })]
pub struct UnityFSSerializedPPtr {
    pub file_id: u32,
    #[br(if(version < 14), temp)]
    legacy_path_id: u32,
    #[br(if(version >= 14, legacy_path_id as _))]
    pub path_id: u64,
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnityAlignedString(pub Vec<u8>);
impl BinRead for UnityAlignedString {
    type Args<'a> = ();
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        use crate::util::SeekExt;
        let len = u32::read_options(reader, endian, ())?;
        let str = <Vec<u8>>::read_options(reader, endian, binrw::args! { count: len as _ })?;
        reader.align_seek(4)?;
        Ok(Self(str))
    }
}
impl std::fmt::Debug for UnityAlignedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UnityAlignedString(\"")?;
        crate::util::display_utf8(&self.0, f, str::escape_debug)?;
        write!(f, "\")")
    }
}
impl std::fmt::Display for UnityAlignedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::util::display_utf8(&self.0, f, str::chars)
    }
}

#[derive(BinRead, Debug)]
#[br(import { version: u32, target_platform: u32 })]
pub struct UnityFSSerializedNamedObjectInfo {
    #[br(if(target_platform == UnityBuildTarget::NoTarget as u32))]
    pub obj_hide_flags: u32,
    #[br(if(target_platform == UnityBuildTarget::NoTarget as u32), args { version })]
    pub prefab_parent_obj: UnityFSSerializedPPtr,
    #[br(if(target_platform == UnityBuildTarget::NoTarget as u32), args { version })]
    pub prefab_internal: UnityFSSerializedPPtr,
    pub name: UnityAlignedString,
}

#[derive(BinRead, Debug)]
#[br(import { version: u32 })]
pub struct UnityFSSerializedAssetBundleObjectAssetInfo {
    pub preload_index: u32,
    pub preload_size: u32,
    #[br(args { version })]
    pub asset_pptr: UnityFSSerializedPPtr,
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32, target_platform: u32 })]
pub struct UnityFSSerializedAssetBundleObjectInfo {
    #[br(args { version, target_platform })]
    pub base: UnityFSSerializedNamedObjectInfo,
    #[br(temp)]
    preload_table_size: u32,
    #[br(count = preload_table_size, args { inner: binrw::args! { version } })]
    pub preload_table: Vec<UnityFSSerializedPPtr>,
    #[br(parse_with = parse_asset_bundle_containers, args { version })]
    pub containers: BTreeMap<UnityAlignedString, Vec<UnityFSSerializedAssetBundleObjectAssetInfo>>,
}

#[derive(NamedArgs)]
pub struct UnityFSVersionArgs {
    pub version: u32,
}

#[binrw::parser(reader, endian)]
fn parse_asset_bundle_containers(
    args: UnityFSVersionArgs,
    ...
) -> binrw::BinResult<BTreeMap<UnityAlignedString, Vec<UnityFSSerializedAssetBundleObjectAssetInfo>>>
{
    let mut map: BTreeMap<_, Vec<UnityFSSerializedAssetBundleObjectAssetInfo>> = BTreeMap::new();
    let len = u32::read_options(reader, endian, ())?;
    for _ in 0..len {
        map.entry(UnityAlignedString::read_options(reader, endian, ())?)
            .or_default()
            .push(UnityFSSerializedAssetBundleObjectAssetInfo::read_options(
                reader,
                endian,
                binrw::args! { version: args.version },
            )?);
    }
    Ok(map)
}

#[derive(BinRead, Debug)]
#[br(import { version: u32 })]
pub struct UnityFSSerializedSpriteObjectInfo {
    // TODO...
}

#[binrw::parser(reader)]
fn read_u8_as_bool() -> binrw::BinResult<bool> {
    Ok(u8::read(reader)? != 0)
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32, engine_ver: UnityEngineVersion })]
pub struct UnityFSSerializedTexture2DObjectTextureSettings {
    pub filter_mode: u32,
    pub aniso: u32,
    pub mip_bias: f32,
    #[br(if(engine_ver >= UnityEngineVersion(2017, 0, 0)))]
    pub wrap_u: u32,
    #[br(if(engine_ver >= UnityEngineVersion(2017, 0, 0)))]
    pub wrap_v: u32,
    #[br(if(engine_ver >= UnityEngineVersion(2017, 0, 0)))]
    pub wrap_w: u32,
    #[br(if(engine_ver < UnityEngineVersion(2017, 0, 0)))]
    pub wrap_mode: u32,
}

#[binread]
#[derive(Debug, Default)]
#[br(import { version: u32, engine_ver: UnityEngineVersion })]
pub struct UnityFSSerializedTexture2DObjectStreamingInfo {
    #[br(if(engine_ver < UnityEngineVersion(2020, 0, 0)), temp)]
    legacy_offset: u32,
    #[br(if(engine_ver >= UnityEngineVersion(2020, 0, 0), legacy_offset as _))]
    pub offset: u64,
    pub size: u32,
    pub path: UnityAlignedString,
}

#[binread]
#[derive(Debug)]
#[br(import { version: u32, target_platform: u32, engine_ver: UnityEngineVersion })]
pub struct UnityFSSerializedTexture2DObjectInfo {
    #[br(args { version, target_platform })]
    pub base: UnityFSSerializedNamedObjectInfo,
    #[br(calc = engine_ver >= UnityEngineVersion(2017, 3, 0), temp)]
    is_2017_3_and_up: bool,
    #[br(if(is_2017_3_and_up))]
    pub forced_fallback_format: u32,
    #[br(if(is_2017_3_and_up), parse_with = read_u8_as_bool)]
    pub downscale_fallback: bool,
    #[br(if(engine_ver >= UnityEngineVersion(2020, 2, 0)), parse_with = read_u8_as_bool)]
    pub is_alpha_channel_optional: bool,
    #[br(temp, align_after = 4)]
    __: (),
    pub width: u32,
    pub height: u32,
    pub complete_image_size: u32,
    #[br(if(engine_ver >= UnityEngineVersion(2020, 0, 0)))]
    pub mips_stripped: u32,
    pub texture_format: u32,
    #[br(if(engine_ver < UnityEngineVersion(5, 2, 0)), parse_with = read_u8_as_bool)]
    pub mipmap: bool,
    #[br(if(engine_ver >= UnityEngineVersion(5, 2, 0)))]
    pub mip_count: u32,
    #[br(parse_with = read_u8_as_bool)]
    pub is_readable: bool,
    #[br(if(engine_ver >= UnityEngineVersion(2020, 0, 0)), parse_with = read_u8_as_bool)]
    pub is_preprocessed: bool,
    #[br(if(engine_ver >= UnityEngineVersion(2019, 3, 0)), parse_with = read_u8_as_bool)]
    pub ignore_master_texture_limit: bool,
    #[br(if(engine_ver < UnityEngineVersion(5, 4, 0)), parse_with = read_u8_as_bool)]
    pub read_allowed: bool,
    #[br(if(engine_ver >= UnityEngineVersion(2018, 2, 0)), parse_with = read_u8_as_bool)]
    pub streaming_mipmaps: bool,
    #[br(temp, align_after = 4)]
    __: (),
    #[br(if(engine_ver >= UnityEngineVersion(2018, 2, 0)))]
    pub streaming_mipmaps_priority: u32,
    pub images_count: u32,
    pub texture_dimension: u32,
    #[br(args { version, engine_ver })]
    pub texture_settings: UnityFSSerializedTexture2DObjectTextureSettings,
    pub lightmap_format: u32,
    #[br(if(engine_ver >= UnityEngineVersion(3, 5, 0)))]
    pub color_space: u32,
    #[br(if(engine_ver >= UnityEngineVersion(2020, 2, 0)), temp)]
    platform_blob_size: u32,
    #[br(if(engine_ver >= UnityEngineVersion(2020, 2, 0)), count = platform_blob_size, align_after = 4)]
    pub platform_blob: Vec<u8>,
    pub image_data_size: u32,
    #[br(if(image_data_size == 0 && engine_ver >= UnityEngineVersion(5, 3, 0)), args { version, engine_ver })]
    pub stream_data: UnityFSSerializedTexture2DObjectStreamingInfo,
}

#[derive(Debug)]
pub enum UnityFSSerializedObject {
    // TODO...
    AssetBundle(UnityFSSerializedAssetBundleObjectInfo),
    Sprite(UnityFSSerializedSpriteObjectInfo),
    Texture2D(UnityFSSerializedTexture2DObjectInfo),
}

#[derive(NamedArgs)]
pub struct UnityFSSerializedObjectArgs {
    pub class_id: u32,
    pub version: u32,
    pub target_platform: u32,
    pub engine_ver: UnityEngineVersion,
}
impl BinRead for UnityFSSerializedObject {
    type Args<'a> = UnityFSSerializedObjectArgs;
    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        use num_traits::FromPrimitive;
        use UnityObjectClassIDType::*;
        let UnityFSSerializedObjectArgs {
            class_id,
            version,
            target_platform,
            engine_ver,
        } = args;
        let obj_class_id = match <UnityObjectClassIDType as FromPrimitive>::from_u32(class_id) {
            Some(x) => x,
            None => {
                return Err(binrw::Error::AssertFail {
                    pos: reader.stream_position()?,
                    message: format!("invalid object class id {}", args.class_id),
                })
            }
        };
        Ok(match obj_class_id {
            AssetBundle => Self::AssetBundle(UnityFSSerializedAssetBundleObjectInfo::read_options(
                reader,
                endian,
                binrw::args! { version, target_platform },
            )?),
            Texture2D => Self::Texture2D(UnityFSSerializedTexture2DObjectInfo::read_options(
                reader,
                endian,
                binrw::args! { version, target_platform, engine_ver },
            )?),
            _ => {
                return Err(binrw::Error::AssertFail {
                    pos: reader.stream_position()?,
                    message: format!(
                        "unsupported object class id {} ({})",
                        args.class_id, obj_class_id
                    ),
                })
            }
        })
    }
}
