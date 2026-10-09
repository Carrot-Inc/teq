package cells

import scala.quoted.*

// A class whose member's inferred type comes from the other file, and a macro that constructs
// the other file's class while the typer runs: each worker's check of a class can wait on the
// other worker, and each worker's macro can demand the other file's class checked meanwhile.
object A:
  def n = B.m + 1
  final class K:
    val shown = "k" + B.m
  def implA(using Quotes): Expr[String] = Expr(new B.L().shown)

inline def fromA: String = ${ A.implA }
