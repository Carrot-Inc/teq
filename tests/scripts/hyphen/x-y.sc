// A script whose name is no identifier: its wrapper class and object are named encoded, as Scala CLI's
// ClassCodeWrapper writes them (Name.backticked), the object reached by its source name too through the package's
// scope (dotty's PackageScope), and it is the entry point --main names by its binary name, as --main-class does.
println(this.getClass.getName)
println(`x-y_sc`.getClass.getName)
