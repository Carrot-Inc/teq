// A case class's copy with named arguments written out of parameter order, as dotty's
// `Application` lifts them (Applications.scala 1219): nothing where every argument is safe (a pure
// path, a synthetic case `apply` of pure arguments, the receiver's fields the copy reads), else
// only what the lifter takes, `LiftComplex` for a method with defaults leaving pure paths in place.
final case class Draft(id: Int, description: String, image: Option[String], effect: Int, tags: List[String])

object Edits:
  def image(c: Draft, value: String): Draft = c.copy(image = Some(value))
  def effect(c: Draft, v: Int): Draft = c.copy(effect = v + 1, id = c.id)
  def describe(c: Draft, s: String): Draft = c.copy(description = s, id = c.id)
  def tag(c: Draft, t: String): Draft = c.copy(tags = List(t), image = None)
  def both(c: Draft, s: String, v: Int): Draft = c.copy(image = Some(s), description = s, effect = v)
  def bump(c: Draft): Draft = c.copy(effect = c.effect + 1, description = c.description)
  def swap(c: Draft): Draft = c.copy(tags = c.tags.reverse, image = c.image)
  def clear(c: Draft): Draft = c.copy(tags = Nil, effect = 0, image = None, description = "")
  def renumber(c: Draft, n: Int): Draft = c.copy(effect = n, id = n)
  def retitle(c: Draft, s: String): Draft = c.copy(description = s.trim, image = Some(s))

@main def run(): Unit =
  val d = Draft(1, "first", None, 2, List("a", "b"))
  println(Edits.image(d, "x"))
  println(Edits.effect(d, 3))
  println(Edits.describe(d, "second"))
  println(Edits.tag(d, "t"))
  println(Edits.both(d, "s", 5))
  println(Edits.bump(d))
  println(Edits.swap(d))
  println(Edits.clear(d))
  println(Edits.renumber(d, 7))
  println(Edits.retitle(d, " r "))
