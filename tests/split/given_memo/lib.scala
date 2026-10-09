package memo

trait TC[A]:
  def name: String

final class Named[A](val name: String) extends TC[A]

trait Low[A]:
  given low: TC[A] = Named("low")

final class Key
object Key extends Low[Key]

object Instances:
  given intTC: TC[Int] = Named("int")
  given strTC: TC[String] = Named("string")

object Others:
  given intTC: TC[Int] = Named("other int")
  given strTC: TC[String] = Named("other string")
