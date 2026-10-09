package ira.inner

object Lib:
  val n: Int = 1
  def m: String = "m"

// A file import whose path starts in the file's own package: `Lib` is `ira.inner.Lib`.
import Lib.n

object Same:
  def f: Int = 0
