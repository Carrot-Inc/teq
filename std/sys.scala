// scala-library's package `scala.sys`, whose package object library bodies name
// (`scala.sys.package.error`).
package scala
package sys

def error(message: String): Nothing = runtimeError(message)

// scala-library's: `System.exit`, which does not return.
def exit(status: Int): Nothing =
  java.lang.System.exit(status)
  throw new IllegalStateException("System.exit returned")

def exit(): Nothing = exit(0)

// The environment of the process: a macro's is the build's, a program's under `teq interp` its own,
// JavaScript's is empty.
def env: collection.immutable.Map[String, String] =
  val entries = java.lang.System.getenv().entrySet().iterator()
  var out = collection.immutable.Map.empty[String, String]
  while entries.hasNext do
    val e = entries.next()
    out = out.updated(e.getKey, e.getValue)
  out

// scala-library's `sys.props`, the system properties as a map: a macro's are the build's and a
// program's under `teq interp` its process's (`System.getProperty`), JavaScript's as Scala.js has
// them.
def props: SystemProperties = new SystemProperties

class SystemProperties:
  def get(key: String): Option[String] = Option(java.lang.System.getProperty(key))
  // Null for a property that is not set: scala-library's `default` of the map.
  def apply(key: String): String = get(key).getOrElse(null)
  def getOrElse(key: String, default: => String): String = get(key).getOrElse(default)
  def contains(key: String): Boolean = get(key).isDefined
