import scala.annotation.targetName

// Two methods of one class under one `@targetName` that their erasures tell apart, as tyrian's
// generated tags declare `button[M](children: List[Elem[M]])` and `button[M]` (dotty's
// `Denotation.matches` holds of neither pair, so neither is a double definition): each is called,
// and a subclass overrides the parameterless one.
trait Tags:
  @targetName("button-no_attrs-list")
  def button[M](children: List[M]): String = s"button with ${children.size} children"
  @targetName("button-no_attrs-list")
  def button[M]: String = "empty button"
  def button[M](label: String): String = s"button $label"

object Plain extends Tags

object Styled extends Tags:
  @targetName("button-no_attrs-list")
  override def button[M]: String = "styled " + super.button[M]

@main def run(): Unit =
  println(Plain.button[Int](List(1, 2)))
  println(Plain.button[Int])
  println(Plain.button[Int]("ok"))
  val tags: Tags = Styled
  println(tags.button[String])
  println(tags.button[String](List("a")))
