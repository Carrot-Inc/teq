// The mirror class of a script's wrapper object, whose name is no identifier: found by the encoded name
// Class.forName takes on the JVM (dotty's mirrorClassBTypeFromSymbol), as Scala CLI's run finds it.
println(`x-y_sc`.getClass.getName)
println(Class.forName("x$minusy_sc").getName)
