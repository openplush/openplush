use openplush_mesh::id_newtype;

id_newtype! {
    /// A chart: a connected region of faces between seams, numbered by its lowest
    /// `FaceId`. Stable for one `SeamSet`.
    ChartId
}

id_newtype! {
    /// A seam chain: a maximal path of seam edges between non-chain
    /// nodes, numbered by its lowest `EdgeId`. Stable for one `SeamSet`.
    SeamChainId
}
