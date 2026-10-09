// Adapted from scala3 tests/run/t3038b.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
class A {
    val a1 = 1
    val a2 = 2
    private val b1 = 3
    private val b2 = 4
    @transient val c1 = 5
    @transient val c2 = 6
    def run = {
        println(a1)
        println(a2)
        println(b1)
        println(b2)
        println(c1)
        println(c2)
    }
}

object Test {
    def main(args: Array[String]): Unit = ()
    new A().run
}
