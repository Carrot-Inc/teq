// `try e catch handler` with a `PartialFunction` handler is dotty's `makeTryCase` (Desugar.scala 2457-2470):
// `case ex$ => { val $N = handler; if $N.isDefinedAt(ex$) then $N(ex$) else throw ex$ }`. teq's lowering applies the
// handler through `applyOrElse` with a rethrowing closure, so it holds no `$N` and binds the closure's parameter
// instead. Pinned to M10 (the tree's shape); the binder `ex$`
// (`nme.DEFAULT_EXCEPTION_NAME`) is dotty's since M2e.
val pf: PartialFunction[Throwable, Int] = { case _: IllegalStateException => 2 }
@main def run(): Unit =
  println(Shape.names { try throw new IllegalStateException("x") catch pf })
