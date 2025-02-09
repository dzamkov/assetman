use assetman::{AssetLoadError, AssetLoadResult, AssetLoader, AssetPath};
use assetman_image::{AssetLoaderImageExt, DynamicImage};
use assetman_json::AssetLoaderJsonExt;
use serdere::{Deserialize, Utf8Reader};
use serdere_json::{JsonDeserializer, ValueExt};
use std::cell::OnceCell;
use std::io::{BufReader, Read};

/// Contains GLTF-loading extensions for [`AssetLoader`].
pub trait AssetLoaderGltfExt {
    /// Loads a GLTF or GLB file.
    fn load_gltf(&self, asset: &AssetPath) -> AssetLoadResult<Gltf<'_>>;
}

impl AssetLoaderGltfExt for AssetLoader<'_> {
    fn load_gltf(&self, asset: &AssetPath) -> AssetLoadResult<Gltf<'_>> {
        match asset.extension() {
            None | Some("gltf") => self.load_json_with(asset, |value| {
                let info: GltfInfo = value.get()?;
                let num_buffers = info.buffers.len();
                Ok(Gltf {
                    assets: self.clone(),
                    dir: asset.parent().unwrap(),
                    info,
                    buffer_cache: (0..num_buffers).map(|_| OnceCell::new()).collect(),
                })
            }),
            Some("glb") => {
                let mut file = self.open_file(asset)?;
                assetman::with_asset(asset, || {
                    let mut header = [0u8; 12];
                    let Ok(()) = file.read_exact(&mut header) else {
                        return Err(MalformedGlbError.into());
                    };
                    if u32::from_le_bytes(header[0..4].try_into().unwrap()) != 0x46546c67 {
                        return Err(MalformedGlbError.into());
                    }
                    let mut chunk_header = [0u8; 8];
                    let Ok(()) = file.read_exact(&mut chunk_header) else {
                        return Err(MalformedGlbError.into());
                    };
                    if u32::from_le_bytes(chunk_header[4..8].try_into().unwrap()) != 0x4e4f534a {
                        return Err(MalformedGlbError.into());
                    }
                    let chunk_len = u32::from_le_bytes(chunk_header[0..4].try_into().unwrap());
                    let mut take_file = file.take(chunk_len as u64);
                    let json_reader =
                        Utf8Reader::new(BufReader::<&mut dyn Read>::new(&mut take_file))?;
                    let info: GltfInfo = serdere::Value::with(
                        &mut serdere_json::TextDeserializer::new(
                            serdere_json::TextDeserializerConfig::strict(),
                            json_reader,
                        )?,
                        |value| value.get(),
                    )?;
                    let num_buffers = info.buffers.len();
                    let res = Gltf {
                        assets: self.clone(),
                        dir: asset.parent().unwrap(),
                        info,
                        buffer_cache: (0..num_buffers).map(|_| OnceCell::new()).collect(),
                    };
                    let mut file = take_file.into_inner();
                    if let Ok(()) = file.read_exact(&mut chunk_header) {
                        if u32::from_le_bytes(chunk_header[4..8].try_into().unwrap()) != 0x004e4942
                        {
                            return Err(MalformedGlbError.into());
                        }
                        let chunk_len = u32::from_le_bytes(chunk_header[0..4].try_into().unwrap());
                        let mut chunk_data = vec![0u8; chunk_len as usize].into_boxed_slice();
                        file.read_exact(&mut chunk_data)?;
                        if num_buffers > 0 && res.info.buffers[0].uri.is_none() {
                            res.buffer_cache[0].set(chunk_data).unwrap();
                        }
                    }
                    Ok(res)
                })
            }
            _ => Err(AssetLoadError {
                asset: asset.clone(),
                inner: UnsupportedExtensionError.into(),
            }),
        }
    }
}

/// The type of error produced when there is an attempt to load a GLTF content from an asset with
/// an unsupported extension.
#[derive(Debug, thiserror::Error)]
#[error("unsupported GLTF file extension")]
pub struct UnsupportedExtensionError;

/// The type of error produced when there is an attempt to load a malformed GLB file.
#[derive(Debug, thiserror::Error)]
#[error("malformed GLB file")]
pub struct MalformedGlbError;

/// Describes the contents of a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct GltfInfo {
    /// The default scene to display.
    pub scene: Option<SceneId>,

    /// The scenes defined in the GLTF file.
    #[serde(default)]
    pub scenes: Vec<SceneInfo>,

    /// The nodes defined in the GLTF file.
    #[serde(default)]
    pub nodes: Vec<NodeInfo>,

    /// The meshes defined in the GLTF file.
    #[serde(default)]
    pub meshes: Vec<MeshInfo>,

    /// The accessors defined in the GLTF file.
    #[serde(default)]
    pub accessors: Vec<AccessorInfo>,

    /// The buffer views defined in the GLTF file.
    #[serde(rename = "bufferViews")]
    #[serde(default)]
    pub buffer_views: Vec<BufferViewInfo>,

    /// The buffers defined in the GLTF file.
    #[serde(default)]
    pub buffers: Vec<BufferInfo>,

    /// The materials defined in the GLTF file.
    #[serde(default)]
    pub materials: Vec<MaterialInfo>,

    /// The textures defined in the GLTF file.
    #[serde(default)]
    pub textures: Vec<TextureInfo>,

    /// The images defined in the GLTF file.
    #[serde(default)]
    pub images: Vec<ImageInfo>,
}

/// Identifies a scene in a GLTF file.
pub type SceneId = u32;

/// Describes a scene in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct SceneInfo {
    /// The name of the scene.
    pub name: Option<String>,

    /// The nodes in the scene.
    #[serde(default)]
    pub nodes: Vec<NodeId>,
}

/// Identifies a node in a GLTF file.
pub type NodeId = u32;

/// Describes a node in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct NodeInfo {
    /// The name of the node.
    pub name: Option<String>,

    /// The translation component of the node's transform.
    pub translation: Option<[f32; 3]>,

    /// The rotation component of the node's transform.
    pub rotation: Option<[f32; 4]>,

    /// The scale component of the node's transform.
    pub scale: Option<[f32; 3]>,

    /// The matrix component of the node's transform.
    pub matrix: Option<[f32; 16]>,

    /// The children of this node.
    #[serde(default)]
    pub children: Vec<NodeId>,

    /// The mesh displayed by this node.
    pub mesh: Option<MeshId>,
}

/// Identifies a mesh in a GLTF file.
pub type MeshId = u32;

/// Describes a mesh in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct MeshInfo {
    /// The name of the mesh.
    pub name: Option<String>,

    /// The primitives which make up the mesh.
    pub primitives: Vec<PrimitiveInfo>,
}

/// Describes a primitive in a [`Mesh`].
#[derive(Debug, Deserialize, Clone)]
pub struct PrimitiveInfo {
    /// The attribute accessors of the primitive.
    pub attributes: AttributeMap,

    /// The indices accessor of the primitive.
    pub indices: Option<AccessorId>,

    /// The material applied to this primitive.
    pub material: Option<MaterialId>,

    /// The type of topology used by this primitive.
    #[serde(default)]
    pub mode: PrimitiveMode,
}

/// The attributes of a primitive.
#[derive(Default, Debug, Clone)]
pub struct AttributeMap {
    /// The accessor for position data.
    pub position: Option<AccessorId>,

    /// The accessor for normal data.
    pub normal: Option<AccessorId>,

    /// The accessor for tangent data.
    pub tangent: Option<AccessorId>,

    /// The accessor for the first set of texture coordinates.
    pub tex_coord_0: Option<AccessorId>,
}

impl AttributeMap {
    /// Gets the accessor corresponding to the given texture coordinate set.
    pub fn tex_coord(&self, id: TextureCoordId) -> Option<AccessorId> {
        match id {
            0 => self.tex_coord_0,
            _ => None,
        }
    }
}

impl<D: JsonDeserializer + ?Sized, Ctx: ?Sized> Deserialize<D, Ctx> for AttributeMap {
    const NULLABLE: bool = false;
    fn deserialize(value: serdere::Value<D>, _: &mut Ctx) -> Result<Self, D::Error> {
        let mut map = AttributeMap::default();
        let mut s_map = value.into_object()?;
        while let Some(mut entry) = s_map.next_entry()? {
            let slot = match &*entry.key()? {
                "POSITION" => &mut map.position,
                "NORMAL" => &mut map.normal,
                "TANGENT" => &mut map.tangent,
                "TEXCOORD_0" => &mut map.tex_coord_0,
                _ => continue,
            };
            *slot = Some(entry.value()?.get()?);
        }
        Ok(map)
    }
}

/// The type of topology used by a [`Primitive`].
#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveMode {
    Points = 0,
    Lines = 1,
    LineLoop = 2,
    LineStrip = 3,
    #[default]
    Triangles = 4,
    TriangleStrip = 5,
    TriangleFan = 6,
}

/// Identifies an accessor in a GLTF file.
pub type AccessorId = u32;

/// Describes an accessor in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct AccessorInfo {
    /// The buffer view containing the data.
    #[serde(rename = "bufferView")]
    pub buffer_view: Option<BufferViewId>,

    /// The byte offset into the buffer view.
    #[serde(rename = "byteOffset")]
    #[serde(default)]
    pub byte_offset: u64,

    /// The type of components in the accessor.
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,

    /// Specifies whether integer data values are normalized before usage.
    #[serde(default)]
    pub normalized: bool,

    /// The number of elements referenced by this accessor.
    pub count: u64,

    /// The type of elements in the accessor.
    #[serde(rename = "type")]
    pub ty: ElementType,
}

/// Identifies the type of components in an [`Accessor`].
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    #[serde(reindex = 5120)]
    Byte,
    #[serde(reindex = 5121)]
    UnsignedByte,
    #[serde(reindex = 5122)]
    Short,
    #[serde(reindex = 5123)]
    UnsignedShort,
    #[serde(reindex = 5125)]
    UnsignedInt,
    #[serde(reindex = 5126)]
    Float,
}

impl ComponentType {
    /// The size, in bytes, of a single component of this type.
    pub fn size(self) -> usize {
        match self {
            ComponentType::Byte | ComponentType::UnsignedByte => 1,
            ComponentType::Short | ComponentType::UnsignedShort => 2,
            ComponentType::UnsignedInt | ComponentType::Float => 4,
        }
    }
}

/// Identifies the type of elements in an [`Accessor`].
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    #[serde(rename = "SCALAR")]
    Scalar,
    #[serde(rename = "VEC2")]
    Vector2,
    #[serde(rename = "VEC3")]
    Vector3,
    #[serde(rename = "VEC4")]
    Vector4,
    #[serde(rename = "MAT2")]
    Matrix2,
    #[serde(rename = "MAT3")]
    Matrix3,
    #[serde(rename = "MAT4")]
    Matrix4,
}

impl ElementType {
    /// The number of components in an element of this type.
    pub fn num_components(self) -> usize {
        match self {
            ElementType::Scalar => 1,
            ElementType::Vector2 => 2,
            ElementType::Vector3 => 3,
            ElementType::Vector4 => 4,
            ElementType::Matrix2 => 4,
            ElementType::Matrix3 => 9,
            ElementType::Matrix4 => 16,
        }
    }
}

/// Identifies a buffer view in a GLTF file.
pub type BufferViewId = u32;

/// Describes a buffer view in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct BufferViewInfo {
    /// The name of the buffer view.
    pub name: Option<String>,

    /// The buffer containing the data.
    pub buffer: BufferId,

    /// The byte offset into the buffer.
    #[serde(rename = "byteOffset")]
    #[serde(default)]
    pub byte_offset: u64,

    /// The number of bytes in the buffer view.
    #[serde(rename = "byteLength")]
    pub byte_length: u64,

    /// The stride between elements in the buffer view.
    #[serde(rename = "byteStride")]
    pub byte_stride: Option<u64>,
}

/// Identifies a buffer in a GLTF file.
pub type BufferId = u32;

/// Describes a buffer in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct BufferInfo {
    /// The name of the buffer.
    pub name: Option<String>,

    /// The URI of the buffer.
    pub uri: Option<String>,

    /// The number of bytes in the buffer.
    #[serde(rename = "byteLength")]
    pub byte_length: u64,
}

/// Identifies a material in a GLTF file.
pub type MaterialId = u32;

/// Describes a material in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct MaterialInfo {
    /// The name of the material.
    pub name: Option<String>,

    /// The PBR parameters for the material.
    #[serde(rename = "pbrMetallicRoughness")]
    pub pbr_metallic_roughness: Option<PbrMetallicRoughnessInfo>,
}

/// A set of parameters for a PBR material.
#[derive(Debug, Deserialize, Clone)]
pub struct PbrMetallicRoughnessInfo {
    /// The base color texture.
    #[serde(rename = "baseColorTexture")]
    pub base_color_texture: Option<TextureRef>,

    /// The metallic-roughness texture.
    #[serde(rename = "metallicRoughnessTexture")]
    pub metallic_roughness_texture: Option<TextureRef>,
}

/// A reference to a texture in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct TextureRef {
    /// The identifier for the texture.
    #[serde(rename = "index")]
    pub texture: TextureId,

    /// The identifier of the texture coordinate set to use.
    #[serde(rename = "texCoord")]
    #[serde(default)]
    pub coord: TextureCoordId,
}

/// Identifies a texture in a GLTF file.
pub type TextureId = u32;

/// Identifies a set of texture coordinates in a GLTF file.
pub type TextureCoordId = u32;

/// Describes a texture in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct TextureInfo {
    /// The name of the texture.
    pub name: Option<String>,

    /// The image used by the texture.
    pub source: Option<ImageId>,
}

/// Identifies an image in a GLTF file.
pub type ImageId = u32;

/// Describes an image in a GLTF file.
#[derive(Debug, Deserialize, Clone)]
pub struct ImageInfo {
    /// The name of the image.
    pub name: Option<String>,

    /// The URI of the image.
    pub uri: Option<String>,

    /// The MIME type of the image.
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,

    /// The buffer view containing the image data.
    #[serde(rename = "bufferView")]
    pub buffer_view: Option<BufferViewId>,
}

/// An instantiation of a GLTF or GLB file.
///
/// Internally contains an [`AssetLoader`] which can be used to load referenced resources on
/// demand.
pub struct Gltf<'a> {
    assets: AssetLoader<'a>,
    dir: AssetPath,
    info: GltfInfo,
    buffer_cache: Box<[OnceCell<Box<[u8]>>]>,
}

impl Gltf<'_> {
    /// Gets the [`GltfInfo`] for this GLTF file.
    pub fn info(&self) -> &GltfInfo {
        &self.info
    }

    /// Gets the default scene to display.
    pub fn scene(&self) -> Option<Scene> {
        let id = self.info.scene?;
        Some(Scene {
            gltf: self,
            info: &self.info.scenes[id as usize],
        })
    }

    /// Iterates over all nodes belonging to `scene()`, if it exists.
    pub fn scene_nodes(&self) -> impl Iterator<Item = Node> {
        let node_ids = self
            .info
            .scene
            .map(|id| &*self.info.scenes[id as usize].nodes)
            .unwrap_or(&[]);
        let gltf = self;
        node_ids.iter().map(move |&id| Node {
            gltf,
            id,
            info: &gltf.info.nodes[id as usize],
        })
    }

    /// Gets the [`Accessor`] with the given identifier.
    pub fn accessor<T>(&self, id: AccessorId) -> Option<Accessor<T>>
    where
        T: Element,
    {
        let info = self.info.accessors.get(id as usize)?;
        if info.ty == T::TYPE {
            Some(Accessor {
                gltf: self,
                info,
                _marker: std::marker::PhantomData,
            })
        } else {
            None
        }
    }

    /// Gets the data for the given buffer.
    pub fn buffer(&self, id: BufferId) -> AssetLoadResult<&[u8]> {
        let cache = &self.buffer_cache[id as usize];
        let mut err = None;
        let res = cache.get_or_init(|| {
            let buffer_info = &self.info.buffers[id as usize];
            let uri = buffer_info.uri.as_ref().expect("buffer has no URI");
            match self.assets.load_bytes(&self.dir.relative(uri)) {
                Ok(data) => data,
                Err(e) => {
                    err = Some(e);
                    Box::new([])
                }
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(res.as_ref()),
        }
    }

    /// Gets the data and stride for the given buffer view.
    pub fn buffer_view(&self, id: BufferViewId) -> AssetLoadResult<(&[u8], Option<usize>)> {
        let buffer_view = &self.info.buffer_views[id as usize];
        let buffer_data = self.buffer(buffer_view.buffer)?;
        let buffer_data = &buffer_data[buffer_view.byte_offset as usize..];
        let buffer_data = &buffer_data[..buffer_view.byte_length as usize];
        Ok((buffer_data, buffer_view.byte_stride.map(|s| s as usize)))
    }

    /// Iterates over the nodes in this GLTF file that have the given name.
    pub fn search_nodes_by_name<'a: 'b, 'b>(
        &'a self,
        name: &'b str,
    ) -> impl Iterator<Item = Node<'a>> + 'b {
        self.info
            .nodes
            .iter()
            .enumerate()
            .filter_map(move |(index, info)| {
                if let Some(mesh_name) = &info.name {
                    if mesh_name == name {
                        Some(Node {
                            gltf: self,
                            id: index as NodeId,
                            info,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    }
}

/// Represents a scene in a [`Gltf`].
pub struct Scene<'a> {
    gltf: &'a Gltf<'a>,
    info: &'a SceneInfo,
}

impl<'a> Scene<'a> {
    /// Gets the [`SceneInfo`] for this scene.
    pub fn info(&self) -> &SceneInfo {
        self.info
    }

    /// Gets the set of nodes in this scene.
    pub fn nodes(&self) -> impl Iterator<Item = Node<'a>> {
        let gltf = self.gltf;
        self.info.nodes.iter().map(move |&id| Node {
            gltf,
            id,
            info: &gltf.info.nodes[id as usize],
        })
    }
}

/// Represents a node in a [`Gltf`].
#[derive(Clone, Copy)]
pub struct Node<'a> {
    gltf: &'a Gltf<'a>,
    id: NodeId,
    info: &'a NodeInfo,
}

impl<'a> Node<'a> {
    /// Gets the [`NodeId`] for this node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Gets the [`NodeInfo`] for this node.
    pub fn info(&self) -> &NodeInfo {
        self.info
    }

    /// Gets the parent of this node, if one exists.
    pub fn parent(&self) -> Option<Node<'a>> {
        let gltf = self.gltf;
        let parent_id = gltf
            .info
            .nodes
            .iter()
            .position(|n| n.children.contains(&self.id))? as NodeId;
        Some(Node {
            gltf,
            id: parent_id,
            info: &gltf.info.nodes[parent_id as usize],
        })
    }

    /// Iterates over the children of this node.
    pub fn children(&self) -> impl Iterator<Item = Node<'a>> {
        let gltf = self.gltf;
        self.info.children.iter().map(move |&id| Node {
            gltf,
            id,
            info: &gltf.info.nodes[id as usize],
        })
    }

    /// Iterates over all descendants of this node, including itself.
    pub fn descendants(&self) -> impl Iterator<Item = Node<'a>> {
        let mut stack = Vec::new();
        stack.push(*self);
        std::iter::from_fn(move || {
            if let Some(node) = stack.pop() {
                for child in node.children() {
                    stack.push(child);
                }
                Some(node)
            } else {
                None
            }
        })
    }

    /// Gets the mesh displayed by this node, if one exists.
    pub fn mesh(&self) -> Option<Mesh<'a>> {
        let gltf = self.gltf;
        let id = self.info.mesh?;
        Some(Mesh {
            gltf,
            info: &gltf.info.meshes[id as usize],
        })
    }
}

/// Represents a mesh in a [`Gltf`].
pub struct Mesh<'a> {
    gltf: &'a Gltf<'a>,
    info: &'a MeshInfo,
}

impl<'a> Mesh<'a> {
    /// Gets the [`MeshInfo`] for this mesh.
    pub fn info(&self) -> &MeshInfo {
        self.info
    }

    /// Gets the primitives which make up this mesh.
    pub fn primitives(&self) -> impl Iterator<Item = Primitive<'a>> {
        let gltf = self.gltf;
        self.info
            .primitives
            .iter()
            .map(move |info| Primitive { gltf, info })
    }
}

/// Represents a primitive in a [`Mesh`].
pub struct Primitive<'a> {
    gltf: &'a Gltf<'a>,
    info: &'a PrimitiveInfo,
}

impl<'a> Primitive<'a> {
    /// Gets the [`PrimitiveInfo`] for this primitive.
    pub fn info(&self) -> &PrimitiveInfo {
        self.info
    }

    /// The [`PrimitiveMode`] of this primitive.
    pub fn mode(&self) -> PrimitiveMode {
        self.info.mode
    }

    /// The [`Material`] for this primitive.
    pub fn material(&self) -> Option<Material<'a>> {
        let gltf = self.gltf;
        let id = self.info.material?;
        Some(Material {
            gltf,
            info: &gltf.info.materials[id as usize],
        })
    }

    /// Gets the [`Accessor`] for the position data of this primitive, if it exists.
    pub fn position(&self) -> Option<Accessor<'a, [f32; 3]>> {
        self.gltf.accessor(self.info.attributes.position?)
    }

    /// Gets the [`Accessor`] for the normal data of this primitive, if it exists.
    pub fn normal(&self) -> Option<Accessor<'a, [f32; 3]>> {
        self.gltf.accessor(self.info.attributes.normal?)
    }

    /// Gets the [`Accessor`] for the texture coordinate data of this primitive corresponding to
    /// the given texture coordinate set, if it exists.
    pub fn tex_coord(&self, id: TextureCoordId) -> Option<Accessor<'a, [f32; 2]>> {
        self.gltf.accessor(self.info.attributes.tex_coord(id)?)
    }

    /// Gets the [`Accessor`] for the indices of this primitive, if they exist.
    pub fn indices(&self) -> Option<Accessor<'a, u32>> {
        self.gltf.accessor(self.info.indices?)
    }
}

/// Represents an accessor in a [`Gltf`] whose elements are logically of type `T`.
pub struct Accessor<'a, T> {
    gltf: &'a Gltf<'a>,
    info: &'a AccessorInfo,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T: Element> Accessor<'a, T> {
    /// Gets an iterator over the elements in this array.
    pub fn elements(&self) -> AssetLoadResult<impl Iterator<Item = T> + 'a> {
        let (data, stride) = self.gltf.buffer_view(self.info.buffer_view.unwrap())?;
        let element_size = T::TYPE.num_components() * self.info.component_type.size();
        let stride = stride.unwrap_or(element_size);
        let mut byte_offset = self.info.byte_offset as usize;
        let component_type = self.info.component_type;
        let normalized = self.info.normalized;
        Ok((0..self.info.count).map(move |_| {
            let element = T::read(
                component_type,
                normalized,
                &data[byte_offset..][..element_size],
            );
            byte_offset += stride;
            element
        }))
    }
}

/// A type which can be used as an element in an [`Accessor`].
pub trait Element: Copy + bytemuck::Pod {
    /// The [`ElementType`] of this element.
    const TYPE: ElementType;

    /// Reads an element of this type from `data`, given that it is encoded with component
    /// type `ty`.
    ///
    /// `data` may exceed the size of the element.
    fn read(ty: ComponentType, normalized: bool, data: &[u8]) -> Self;
}

impl Element for u32 {
    const TYPE: ElementType = ElementType::Scalar;
    fn read(ty: ComponentType, _: bool, data: &[u8]) -> Self {
        match ty {
            ComponentType::Byte => data[0] as u32,
            ComponentType::UnsignedShort => bytemuck::pod_read_unaligned::<u16>(data) as u32,
            ComponentType::UnsignedInt => bytemuck::pod_read_unaligned(data),
            _ => panic!("invalid component type for u32"),
        }
    }
}

impl Element for [f32; 2] {
    const TYPE: ElementType = ElementType::Vector2;
    fn read(ty: ComponentType, _: bool, data: &[u8]) -> Self {
        match ty {
            ComponentType::Float => bytemuck::pod_read_unaligned(&data[0..8]),
            _ => panic!("invalid component type for vec2"),
        }
    }
}

impl Element for [f32; 3] {
    const TYPE: ElementType = ElementType::Vector3;
    fn read(ty: ComponentType, _: bool, data: &[u8]) -> Self {
        match ty {
            ComponentType::Float => bytemuck::pod_read_unaligned(&data[0..12]),
            _ => panic!("invalid component type for vec3"),
        }
    }
}

/// Represents a material in a [`Gltf`].
pub struct Material<'a> {
    gltf: &'a Gltf<'a>,
    info: &'a MaterialInfo,
}

impl<'a> Material<'a> {
    /// Gets the [`MaterialInfo`] for this material.
    pub fn info(&self) -> &MaterialInfo {
        self.info
    }

    /// The name of this material, if available.
    pub fn name(&self) -> Option<&str> {
        self.info.name.as_deref()
    }

    /// Gets the base color texture for this material, if applicable.
    pub fn base_color_texture(&self) -> Option<Texture<'a>> {
        // TODO: Handle texture coordinate
        let gltf = self.gltf;
        let id = self
            .info
            .pbr_metallic_roughness
            .as_ref()?
            .base_color_texture
            .as_ref()?
            .texture;
        let info = &gltf.info.textures[id as usize];
        Some(Texture { gltf, info })
    }
}

/// Represents a texture in a [`Gltf`].
pub struct Texture<'a> {
    gltf: &'a Gltf<'a>,
    info: &'a TextureInfo,
}

impl<'a> Texture<'a> {
    /// Gets the [`TextureInfo`] for this texture.
    pub fn info(&self) -> &TextureInfo {
        self.info
    }

    /// The name of this texture, if available.
    pub fn name(&self) -> Option<&str> {
        self.info.name.as_deref()
    }

    /// The image used by this texture.
    pub fn image(&self) -> Image<'a> {
        let gltf = self.gltf;
        let id = self.info.source.unwrap();
        let info = &gltf.info.images[id as usize];
        Image { gltf, info }
    }
}

/// Represents an image in a [`Gltf`].
pub struct Image<'a> {
    gltf: &'a Gltf<'a>,
    info: &'a ImageInfo,
}

impl Image<'_> {
    /// Gets the [`ImageInfo`] for this image.
    pub fn info(&self) -> &ImageInfo {
        self.info
    }

    /// Gets the dimensions of this image.
    pub fn size(&self) -> AssetLoadResult<[u32; 2]> {
        if let Some(buffer_view) = self.info.buffer_view {
            todo!()
        } else {
            self.gltf
                .assets
                .size_image(&self.gltf.dir.relative(self.info.uri.as_ref().unwrap()))
        }
    }

    /// Gets a portable reference to the source data for this image.
    pub fn source(&self) -> ImageSource {
        if let Some(buffer_view) = self.info.buffer_view {
            todo!()
        } else {
            ImageSource::Asset(self.gltf.dir.relative(self.info.uri.as_ref().unwrap()))
        }
    }
}

/// A portable reference to the source data for an [`Texture`].
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ImageSource {
    Asset(AssetPath),
}

impl ImageSource {
    /// Loads this image.
    pub fn load(&self, assets: &AssetLoader) -> AssetLoadResult<DynamicImage> {
        match self {
            ImageSource::Asset(path) => assets.load_image(path),
        }
    }
}
