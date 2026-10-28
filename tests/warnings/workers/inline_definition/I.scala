object Old { @deprecated("old", "1") def v: Int = 1 }
object I { inline def get: Int = Old.v }
