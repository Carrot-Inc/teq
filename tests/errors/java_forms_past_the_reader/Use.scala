// The Java reader reads classes and interfaces: an enum, a record, an annotation declaration, a sealed type and a member
// type are refused where they are declared, the file kept (no class of the class path stands in for one).
// expect: Ann.java:1:19: error: not supported yet: a Java annotation declaration (the Java reader reads classes and interfaces)
// expect: Enum.java:1:13: error: not supported yet: a Java enum (the Java reader reads classes and interfaces)
// expect: Outer.java:2:23: error: not supported yet: a Java member type (the Java reader reads classes and interfaces)
// expect: Rec.java:1:15: error: not supported yet: a Java record (the Java reader reads classes and interfaces)
// expect: Sealed.java:1:25: error: not supported yet: a sealed Java type (the Java reader reads classes and interfaces)
object Use { def main(args: Array[String]): Unit = println(new Outer) }
