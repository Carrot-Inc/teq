// The givens of a stable value's class come in with `import v.given` (or a given selector), and
// its Scala 2 implicit vals with `import v.*`, which leaves the givens out; each is then read
// on that value.
trait Codec[A]:
  def name: String

class Defs(prefix: String):
  given intCodec: Codec[Int] with
    def name = "int"
  implicit val strCodec: Codec[String] = new Codec[String]:
    def name = s"$prefix-string"
  given Codec[Boolean] = new Codec[Boolean]:
    def name = s"$prefix-bool"
  def plain: Int = 1

def show[A](using c: Codec[A]): String = c.name

@main def run(): Unit =
  val d = Defs("d")
  locally {
    import d.given
    println(summon[Codec[Int]].name)
    println(show[Boolean])
    println(intCodec.name)
  }
  locally {
    import d.*
    println(summon[Codec[String]].name)
    println(plain)
  }
  locally {
    import d.{given Codec[Boolean]}
    println(show[Boolean])
  }
  locally {
    import d.strCodec
    println(show[String])
  }
