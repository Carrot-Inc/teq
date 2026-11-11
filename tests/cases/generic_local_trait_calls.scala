class Outer(base: Int):
  def run(): Unit =
    val captured = base
    def choose[A](first: A)(second: => A): A =
      println(captured)
      second
    class Holder:
      trait Read:
        def number: Int = choose(0)(captured + 1)
        def word: String = choose[String]("")("ok")
      class Inner extends Read
      def result(): Unit =
        val inner = new Inner
        println(inner.number)
        println(inner.word)
    new Holder().result()

@main def run(): Unit = new Outer(10).run()
