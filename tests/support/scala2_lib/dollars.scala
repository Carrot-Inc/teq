package dollars

// Scala 2 traits whose names hold a `$` the source writes (tests/classpath/jvm/scala2_dollar_trait_storage): the
// class mixing one in finds the trait in its pickle by the binary name its owners make (`A$B` is one trait, not `B` in
// `A`), and a nested one's pickle on its owner's class file (`C$D.class` for `C$D$T`, not `C.class`).
object Marker

trait `A$B` {
  private lazy val n: Int = { println("A$B lazy init"); 7 }
  def read: Int = n
}

object `C$D` {
  trait T {
    println("C$D.T init")
    private var count = 1
    object cell { println("C$D.T cell init"); def owner: T = T.this }
    private lazy val twice: Int = { println("C$D.T twice init"); count * 2 }
    def next: Int = { count += 1; count + twice }
  }
}
