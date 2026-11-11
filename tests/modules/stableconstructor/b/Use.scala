package scb

import sca.*

inline def ignore(x: Any): Unit = ()

@main def run(): Unit =
  ignore(new P(1)); println("|")
  ignore(new P2(2)); println("|")
  ignore(new S(3)); println("|")
  ignore(new R(4)); println("|")
  ignore(new N(5)); println("|")
  ignore(new UsesTI); println("|")
  ignore(new UsesTN); println("|")
  ignore(new K(6)); println("|")
  ignore(new M(7)); println(Made.count)
  new P(8)
  println("|")
  new Ext
  new Exporter(Box(1))
  new UsesTExporter
  println("|")
