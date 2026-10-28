object Old { @deprecated("old", "1") def v: Int = 1 }
object B { def value = Old.v }
