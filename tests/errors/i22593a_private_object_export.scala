// dotty's neg/i22593a: a private object of a package exported from it is no member to select from outside
// (E173); the export's forwarders stand. Master accepted it.
import scala.quoted.*

package jam {
  trait JamCoreDsl {
    implicit inline def defaultJamConfig: this.JamConfig =
      new JamConfig(brewRecRegex = ".*")
    class JamConfig(val brewRecRegex: String)
    inline def brew(implicit inline config: JamConfig): Unit = ???
  }
  private object internal extends JamCoreDsl
  export internal._
}

object test {
  jam.brew(jam.internal.defaultJamConfig) // error
  jam.brew // error
}
// expect: 17:12: error: object internal cannot be accessed as a member of jam.type from object test.
