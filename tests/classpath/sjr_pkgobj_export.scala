// jars: scala-library scalajs-react
// scalajs-react 4.0.0's event aliases from its jars: `package object react extends ReactEventTypes`, whose aliases
// the package has as members, exported by name and imported by a wildcard.
object Prelude:
  export japgolly.scalajs.react.{ReactEvent, ReactEventFromInput, ReactKeyboardEventFromHtml}
import Prelude.*

def key(e: ReactKeyboardEventFromHtml): String = e.key
def value(e: ReactEventFromInput): String = e.target.value
def stamp(e: ReactEvent): Double = e.timeStamp

object Wild:
  import japgolly.scalajs.react.*
  def form(e: ReactFormEvent): Boolean = e.defaultPrevented
