// A Java and a Scala class of one name: the Scala source first in the build's order, the Java declaration clashes
// with it (E161), once, as the reference counts it.
// expect: J.java:1:12: error: J clashes with class J in
package p
class J { def n: Int = 7 }
