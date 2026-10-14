// A conversion two packages forward, the package's own top-level forwarder and an imported package's, is
// one candidate: dotty's ContextualImplicits.combineEligibles keeps one of a name per level, a definition
// over an import, a later import over an earlier one (lila's `catsSyntaxOptionId` through lila.ui.* and
// lila.irwin's own export of Lilaism).
package irwin
import ui.*
@main def run(): Unit = println(1.some)
