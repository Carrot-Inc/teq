// jars: scala-library universal-lib
// A loaded trait is universal by its first parent as completed and dealiased (dotty's `Namer.ensureFirstIsClass`), as
// a source trait is: scalac's pickle names the alias (`trait Selected extends Types.Root`), which a test of the
// spelling `scala.Any` took for an `AnyRef`. A trait extending one without `Any` is an `AnyRef`.
// expect: 8:46: error: type mismatch: found Selected, required AnyRef
// expect: 9:36: error: type mismatch: found Top, required AnyRef
object UniversalAliasLib:
  def selected(x: unilib.Selected): AnyRef = x
  def top(x: unilib.Top): AnyRef = x
  def below(x: unilib.Below): AnyRef = x
