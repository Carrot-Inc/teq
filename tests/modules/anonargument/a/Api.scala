package aaa

// An anonymous instance passed as an argument: its class escapes the block it is in, which is
// ascribed the class's bound where that conforms to the parameter's type and the parameter's
// type otherwise (`Typer.ensureNoLocalRefs`, `TypeOps.avoid`, `classBound`).
trait Shape:
  type Measure
  def size: Int

object Api:
  def use(x: AnyRef { def f(x: Int): Int }): String = "refined"
  def wide(x: Any): String = "wide"
  def shape(s: Shape { type Measure = Int }): Int = s.size
  def calls(): String =
    use(new AnyRef { def f(x: Int): Int = x }) + " " +
      wide(new Shape { type Measure = String; def size = 1 }) + " " +
      shape(new Shape { type Measure = Int; def size = 2 })
