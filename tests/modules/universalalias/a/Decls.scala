package uaa

// A trait whose first parent resolves to `Any`, written so or through an alias, is universal and no `AnyRef` (dotty's
// `Namer.ensureFirstIsClass`); its pickle names `Any` first, as scalac's does, so a downstream module reads it so. A
// trait extending one without `Any` gets `Object` first and is an `AnyRef`.
type Root = Any
object Types { type Root = Any }
trait Written extends Any
trait Aliased extends Root
trait Selected extends Types.Root
trait Below extends Aliased
