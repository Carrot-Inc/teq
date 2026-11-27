// A local class's overriding extension method with a default of its own, the downstream leaving it: scalac prints 10.
// The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = { val c = Ext.make(); import c.*; println(2.m()) }
