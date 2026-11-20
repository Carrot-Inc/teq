// A simple type name in a Java class's body that an import resolves where a parent of the class has a member type of
// that name (java.util.Map.Entry, through AbstractMap) is refused at the name once a Scala source reads the class: the
// reader does not look up inherited member types, which javac's and scalac's scope find first. The class's own header
// binds the import's type (Comparable[q.Entry]); a class no Scala source reads (K) is javac's alone.
// expect: J.java:6:29: error: not supported yet: Entry in a class whose parent java.util.Map has the member type java.util.Map.Entry, where the Java reader finds q.Entry (it does not look up inherited member types)
object Use { def f(j: J): Int = j.compareTo(new q.Entry) + J.pick(null).length }
