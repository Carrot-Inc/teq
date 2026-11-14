package jma

// An inline method whose body calls an overloaded Java method over a static member class: the pickled call
// names its alternative by its signature, where the member class stands as a member of its outer class's
// companion (`java.lang.ProcessBuilder$.Redirect`), which is how a module compiled by scalac finds it.
object Sig:
  inline def out(pb: ProcessBuilder, r: ProcessBuilder.Redirect): ProcessBuilder = pb.redirectOutput(r)
