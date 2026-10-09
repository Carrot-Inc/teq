import scala.quoted.*

val boot: Int = inc
inline def marker: Int = ${ State.readImpl }
