// A Java and a Scala class of one name: the Java source first in the build's order, the Scala class is already defined
// and the Java declaration clashes (E161), as the reference counts them.
// expect: J.java:1:12: error: J clashes with class J in
// expect: Use.scala:6:7: error: J is already defined as class J in
package p
class J { def n: Int = 7 }
