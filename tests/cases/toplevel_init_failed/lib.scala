//> using platform js
package fi
// A file whose initialiser throws: Scala.js reports the exception once and runs the def after
// it, which teq's JavaScript output does; the JVM reports ExceptionInInitializerError and then
// NoClassDefFoundError, and teq's interpreter runs the initialiser again on the next access.
val boom: Int = { println("init"); throw new RuntimeException("x") }
def f: Int = 7
