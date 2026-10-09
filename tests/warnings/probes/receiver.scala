// An inherited given is credited to the import of the object the search read it on.
trait Base {
  def v: Int
  given n: Int = v
}
object A extends Base { def v: Int = 1 }
object B extends Base { def v: Int = 2 }
object Use {
  import A.n
  def f: Int = {
    import B.given
    summon[Int]
  }
}
@main def main(): Unit = println(Use.f)
