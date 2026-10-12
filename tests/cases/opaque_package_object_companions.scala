// A top-level object named as a top-level type of its file, an opaque type or an alias, is held by the
// file's package object with the top-level definitions (dotty's Desugar.packageDef), and sees every
// opaque type of it: lila's PatronColorChoice.resolve returns a PatronColorResolved.
enum PatronColor(val id: Int):
  case color1 extends PatronColor(1)
  case color2 extends PatronColor(2)

opaque type PatronColorChoice = PatronColor
object PatronColorChoice:
  extension (color: PatronColorChoice) def value: PatronColor = color
  def apply(color: PatronColor): PatronColorChoice = color
  def resolve(choice: Option[PatronColorChoice], default: PatronColor): PatronColorResolved =
    choice.getOrElse(default)

opaque type PatronColorResolved = PatronColor
object PatronColorResolved:
  extension (color: PatronColorResolved) def value: PatronColor = color

type Pair = (Int, Int)
opaque type Count = Int
object Pair:
  def count(n: Int): Count = n
  def of(c: Count): Int = c + 1

@main def run(): Unit =
  println(PatronColorChoice.resolve(None, PatronColor.color2).value)
  println(PatronColorChoice.resolve(Some(PatronColorChoice(PatronColor.color1)), PatronColor.color2).value)
  println(Pair.of(Pair.count(4)))
