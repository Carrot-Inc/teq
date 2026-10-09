// A Scala 2 implicit and a given are candidates of one search, ranked by the same rules.
trait Codec[A] { def name: String }
object Codec {
  implicit val codecInt: Codec[Int] = new Codec[Int] { def name = "int" }
  given Codec[String] with { def name = "string" }
  implicit def codecList[A](implicit c: Codec[A]): Codec[List[A]] = new Codec[List[A]] { def name = s"list[${c.name}]" }
  given [A](using c: Codec[A]): Codec[Option[A]] with { def name = s"option[${c.name}]" }
}
trait Animal { def sound: String }
class Dog extends Animal { def sound = "woof" }
trait Pretty[-A] { def pretty(a: A): String }
object Pretty {
  implicit val prettyAnimal: Pretty[Animal] = a => s"animal:${a.sound}"
}
object Local {
  implicit val prettyDog: Pretty[Dog] = d => s"dog:${d.sound}"
}
object Scoped {
  def codecOf[A](implicit c: Codec[A]): String = c.name
  def prettyOf[A](a: A)(implicit p: Pretty[A]): String = p.pretty(a)
  def inner(): String = {
    implicit val codecLocal: Codec[Int] = new Codec[Int] { def name = "local-int" }
    codecOf[Int] + " " + codecOf[List[Int]]
  }
}
object WithImports {
  object Instances {
    implicit val codecBool: Codec[Boolean] = new Codec[Boolean] { def name = "bool" }
    given Codec[Double] with { def name = "double" }
    def plain: Int = 1
  }
  def wildcard(): String = {
    import Instances._
    Scoped.codecOf[Boolean] + " " + plain
  }
  def givenImport(): String = {
    import Instances.given
    Scoped.codecOf[Boolean] + " " + Scoped.codecOf[Double]
  }
  def named(): String = {
    import Instances.codecBool
    Scoped.codecOf[List[Boolean]]
  }
}
@main def main(): Unit =
  import Scoped._
  println(codecOf[Int] + " " + codecOf[String] + " " + codecOf[List[Int]] + " " + codecOf[Option[String]] + " " + codecOf[List[Option[Int]]])
  println(prettyOf(new Dog))
  def withLocal(): Unit = {
    import Local._
    println(prettyOf(new Dog))
  }
  withLocal()
  println(inner())
  println(WithImports.wildcard() + " " + WithImports.givenImport() + " " + WithImports.named())
