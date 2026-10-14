package xflagsm

import scala.quoted.*

object Inspect:
  inline def flags: String = ${ flagsImpl }
  def flagsImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val wrap = TypeRepr.of[xflags.Wrap].typeSymbol
    def show(s: Symbol): String =
      val names = List(Flags.Exported -> "exported", Flags.Final -> "final", Flags.Method -> "method", Flags.StableRealizable -> "stable")
      names.collect { case (f, n) if s.flags.is(f) => n }.mkString(s.name + ":", ",", "")
    Expr(List(show(wrap.methodMember("f").head), show(wrap.methodMember("n").head), show(wrap.typeMember("T"))).mkString(" "))
