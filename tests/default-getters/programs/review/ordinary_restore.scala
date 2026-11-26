object Main {
  var x = 3
  trait Ordinary { def m(a: Int = { x += 1; x }): Int = a }
  class Regular { def m(a: Int = { x += 1; x }): Int = a }
  def local(): Unit = {
    var y = 10
    trait T { def m(a: Int = y): Int = a }
    class C extends T
    println(new C().m())
    y = 20
    println(new C().m())
  }
  def main(args: Array[String]): Unit = {
    local()
    println(new Ordinary {}.m())
    println(new Regular().m())
    println(x)
  }
}