import scala.quoted.*

val boot: Int = inc
transparent inline def marker: Int = ${ State.readImpl }
