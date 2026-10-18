// A script in a directory whose name is no identifier: its wrapper's package is the directory's name encoded
// (AmmUtil.encodeScalaSourcePath), and the names the JVM gives encode every part of the path (dotty's javaClassName,
// fullName.mangledString), as --main takes them, as Scala CLI's --main-class does.
println(this.getClass.getName)
println(run_sc.getClass.getName)
