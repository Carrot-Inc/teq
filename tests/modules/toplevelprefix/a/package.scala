package tpa

// Top-level definitions of a file, which a downstream selects on the file's `package$package`
// object: the prefix is written once and shared, the split build's as the whole build's.
type Label = String
def label(s: String): Label = s.trim
val prefix: Label = "p"
