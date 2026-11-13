// Three `given T` selectors in a block whose types name the block's own class, and a plain inline method
// whose `summonInline` the first answers where the inlining phase expands the call: the unused-import check
// credits the selector whose own type the given conforms to. scalac 3.8.4 (-Wunused:imports) warns of the
// second and the third selector and prints `local`.
trait Ranked[T] { def rank: String }
trait Shown { def show: String }
inline def plainRanked[A]: String = scala.compiletime.summonInline[Ranked[A]].rank
object Maker {
  def make(): Shown = {
    class Local
    object Inner {
      given localRanked: Ranked[Local] = new Ranked[Local] { def rank = "local" }
      given listRanked: Ranked[List[Local]] = new Ranked[List[Local]] { def rank = "locals" }
      given other: String = "s"
    }
    import Inner.{given Ranked[Local], given Ranked[List[Local]], given String}
    new Shown { def show = plainRanked[Local] }
  }
}
@main def main(): Unit = println(Maker.make().show)
