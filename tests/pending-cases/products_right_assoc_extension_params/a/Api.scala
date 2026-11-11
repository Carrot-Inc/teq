package rda

object Api:
  extension (r: String) def ++:(a: String)(b: String = a.concat("/").concat(r)): String = b
  def local(): String = ("A" ++: "R")()
  extension (r: String) def --:(a: String): String = a.concat("-").concat(r)
  extension (r: String) inline def ~~:(a: String): String = a.concat("~").concat(r)
