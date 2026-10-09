// `isInstanceOf` of an opaque type: inside its scope the test is the underlying type's; outside,
// where the type is abstract, the test is unchecked and holds for every value of its bound, as
// Magnolia's `v.isInstanceOf[p]` on a field's default does for a field of an opaque type.
object Ids:
  opaque type UserId = Long
  opaque type Name <: String = String
  object UserId:
    def apply(v: Long): UserId = v
    def inside(x: Any): Boolean = x.isInstanceOf[UserId]
  object Name:
    def apply(v: String): Name = v

final case class User(id: Ids.UserId = Ids.UserId(7L), name: Ids.Name = Ids.Name("anon"))

object Main:
  def defaultAs[p](evaluator: () => Any): Option[p] =
    val v = evaluator()
    if (v: @unchecked).isInstanceOf[p] then Some(v.asInstanceOf[p]) else None
  inline def defaultAsInline[p](evaluator: () => Any): Option[p] =
    val v = evaluator()
    if (v: @unchecked).isInstanceOf[p] then Some(v.asInstanceOf[p]) else None
  def main(args: Array[String]): Unit =
    println(Ids.UserId.inside(3L))
    println(Ids.UserId.inside("x"))
    println((7L: Any) match { case _: Ids.UserId @unchecked => "id"; case _ => "other" })
    println(defaultAs[Ids.UserId](() => User().id).isDefined)
    println(defaultAs[Ids.Name](() => User().name))
    println(defaultAs[Ids.Name](() => 12).isDefined)
    println(defaultAs[Int](() => 12).isDefined)
    println(defaultAsInline[Ids.UserId](() => User().id).isDefined)
    println(defaultAsInline[Ids.Name](() => User().name))
    println(defaultAsInline[Ids.Name](() => 12).isDefined)
    println(defaultAsInline[Ids.UserId](() => "s").isDefined)
    println(defaultAsInline[Int](() => "s").isDefined)
