package core

import scala.quoted.*

/** A macro that reads the file system at compile time through `java.nio.file`, which the lean std's javalib runs and
  * a JDK class cannot (no body): a Scala.js project's check under the compiler toggle must type it against the lean std. */
object FileMacros:
  inline def existsAtCompileTime(inline path: String): Boolean = ${ existsImpl('path) }

  private def existsImpl(path: Expr[String])(using Quotes): Expr[Boolean] =
    Expr(java.nio.file.Files.exists(java.nio.file.Path.of(path.valueOrAbort)))
