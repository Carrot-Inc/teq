// jars: sourcecode-0.4.4
// sourcecode 0.4.4, the application's version: its `Line` and `FileName` macros cache per
// source file in a `ConcurrentHashMap` through `computeIfAbsent` with a lambda of the JDK's
// `Function` shape, and count lines with `Iterator.indexWhere(p, from)`.
//> using dep com.lihaoyi::sourcecode:0.4.4
package demo.lines44

object Site:
  def here(implicit line: sourcecode.Line): Int = line.value
  def where(implicit file: sourcecode.FileName, name: sourcecode.Name, enc: sourcecode.Enclosing): String =
    file.value + " " + name.value + " " + enc.value

object Main:
  val first = Site.here
  def main(args: Array[String]): Unit =
    println(first)
    println(Site.here)
    println(Site.where)
    val nested = Site.here +
      Site.here
    println(nested)
