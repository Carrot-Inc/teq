// A named selector of an import through a value whose type is a type parameter or an intersection
// is taken as it was before the selectors were checked (`Checking.checkImportSelectors` checks a
// class's own type here): a nested class or a type member of the bound or of a part
// (`a.Box` for an `A <: API` and for an `API & Other`).
class API:
  class Box
  type Item = Int

trait Other

class UseBound[A <: API](val a: A):
  import a.{Box, Item}
  def item: Item = 3

class UseBoth(val a: API & Other):
  import a.{Box, Item}
  def item: Item = 4

@main def run(): Unit =
  println(new UseBound(new API).item)
  println(new UseBoth(new API with Other).item)
