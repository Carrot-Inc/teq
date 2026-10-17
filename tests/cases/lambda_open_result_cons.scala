// An invariant class's value built in a `map` lambda on the right of `::` under a declared result: the
// lambda's expected result is `map`'s open `B`, so `Item.apply`'s `T`, held nonvariantly, stays open
// (`Inferencing.interpolateTypeVars`; `Namer.valOrDefDefSig` completes an anonymous function's result
// from its contextual type, `Typer.typedFunctionValue`), and `::`'s expected `List[Item[Option[String]]]`
// fixes it to `Option[String]`; a default argument written, or none declared, changes nothing.
package lambdaopenresultcons

final case class Item[T](title: String, value: T, disabled: Boolean = false)
final case class Plain[T](title: String, value: T, disabled: Boolean)
final case class Category(code: String, label: String)
object Category:
  val values: List[Category] = List(Category("flower", "Flower"), Category("edible", "Edible"))

object Main:
  def items(noneLabel: String = "No limit"): List[Item[Option[String]]] =
    Item[Option[String]](noneLabel, None) :: Category.values.map(c => Item(c.label, Some(c.code)))
  def written(noneLabel: String = "No limit"): List[Item[Option[String]]] =
    Item[Option[String]](noneLabel, None) :: Category.values.map(c => Item(c.label, Some(c.code), false))
  def plain(noneLabel: String = "No limit"): List[Plain[Option[String]]] =
    Plain[Option[String]](noneLabel, None, false) :: Category.values.map(c => Plain(c.label, Some(c.code), false))
  def main(args: Array[String]): Unit =
    println(items().map(_.value))
    println(written().map(_.value))
    println(plain("none").map(_.value))
