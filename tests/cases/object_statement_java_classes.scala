// teq: --wconf msg=A.pure.expression.does.nothing:s
// An object that extends a class of the JDK is evaluated where it is named as a block's statement, whatever the
// class's body is here: to the reference a Java class always runs an initialiser. One that mixes in a Java
// interface alone is not: an interface runs none. Each object's effect sits where the reference sees no
// initialiser of the object's own: a mixed-in trait's parameter default for a class's row, a secondary
// constructor's argument for an interface's.
trait Reg_ArrayList(name: String = { println("  made: java.util.ArrayList"); "x" })
object O_ArrayList extends java.util.ArrayList[AnyRef] with Reg_ArrayList
trait Reg_HashMap(name: String = { println("  made: java.util.HashMap"); "x" })
object O_HashMap extends java.util.HashMap[AnyRef, AnyRef] with Reg_HashMap
trait Reg_HashSet(name: String = { println("  made: java.util.HashSet"); "x" })
object O_HashSet extends java.util.HashSet[AnyRef] with Reg_HashSet
trait Reg_ArrayDeque(name: String = { println("  made: java.util.ArrayDeque"); "x" })
object O_ArrayDeque extends java.util.ArrayDeque[AnyRef] with Reg_ArrayDeque
trait Reg_TreeSet(name: String = { println("  made: java.util.TreeSet"); "x" })
object O_TreeSet extends java.util.TreeSet[AnyRef] with Reg_TreeSet
trait Reg_Throwable(name: String = { println("  made: java.lang.Throwable"); "x" })
object O_Throwable extends java.lang.Throwable with Reg_Throwable
trait Reg_Exception(name: String = { println("  made: java.lang.Exception"); "x" })
object O_Exception extends java.lang.Exception with Reg_Exception
trait Reg_RuntimeException(name: String = { println("  made: java.lang.RuntimeException"); "x" })
object O_RuntimeException extends java.lang.RuntimeException with Reg_RuntimeException
trait Reg_AtomicInteger(name: String = { println("  made: java.util.concurrent.atomic.AtomicInteger"); "x" })
object O_AtomicInteger extends java.util.concurrent.atomic.AtomicInteger with Reg_AtomicInteger
trait Reg_InputStream(name: String = { println("  made: java.io.InputStream"); "x" })
object O_InputStream extends java.io.InputStream with Reg_InputStream { def read(): Int = -1 }
trait Reg_OutputStream(name: String = { println("  made: java.io.OutputStream"); "x" })
object O_OutputStream extends java.io.OutputStream with Reg_OutputStream { def write(b: Int): Unit = () }
trait Reg_Writer(name: String = { println("  made: java.io.Writer"); "x" })
object O_Writer extends java.io.Writer with Reg_Writer { def write(cbuf: Array[Char], off: Int, len: Int): Unit = (); def flush(): Unit = (); def close(): Unit = () }
trait Reg_ClassLoader(name: String = { println("  made: java.lang.ClassLoader"); "x" })
object O_ClassLoader extends java.lang.ClassLoader with Reg_ClassLoader
trait Reg_ConcurrentLinkedQueue(name: String = { println("  made: java.util.concurrent.ConcurrentLinkedQueue"); "x" })
object O_ConcurrentLinkedQueue extends java.util.concurrent.ConcurrentLinkedQueue[AnyRef] with Reg_ConcurrentLinkedQueue
class S_Runnable(x: Int) { def this() = this({ println("  made: java.lang.Runnable"); 1 }) }
object O_Runnable extends S_Runnable() with java.lang.Runnable { def run(): Unit = () }
class S_Comparable(x: Int) { def this() = this({ println("  made: java.lang.Comparable"); 1 }) }
object O_Comparable extends S_Comparable() with java.lang.Comparable[AnyRef] { def compareTo(o: AnyRef): Int = 0 }
class S_AutoCloseable(x: Int) { def this() = this({ println("  made: java.lang.AutoCloseable"); 1 }) }
object O_AutoCloseable extends S_AutoCloseable() with java.lang.AutoCloseable { def close(): Unit = () }
class S_Closeable(x: Int) { def this() = this({ println("  made: java.io.Closeable"); 1 }) }
object O_Closeable extends S_Closeable() with java.io.Closeable { def close(): Unit = () }
class S_Supplier(x: Int) { def this() = this({ println("  made: java.util.function.Supplier"); 1 }) }
object O_Supplier extends S_Supplier() with java.util.function.Supplier[AnyRef] { def get(): AnyRef = null }
class S_Comparator(x: Int) { def this() = this({ println("  made: java.util.Comparator"); 1 }) }
object O_Comparator extends S_Comparator() with java.util.Comparator[AnyRef] { def compare(a: AnyRef, b: AnyRef): Int = 0 }
class S_Iterable(x: Int) { def this() = this({ println("  made: java.lang.Iterable"); 1 }) }
object O_Iterable extends S_Iterable() with java.lang.Iterable[AnyRef] { def iterator(): java.util.Iterator[AnyRef] = null }
class S_Serializable(x: Int) { def this() = this({ println("  made: java.io.Serializable"); 1 }) }
object O_Serializable extends S_Serializable() with java.io.Serializable
class S_Cloneable(x: Int) { def this() = this({ println("  made: java.lang.Cloneable"); 1 }) }
object O_Cloneable extends S_Cloneable() with java.lang.Cloneable
class S_RandomAccess(x: Int) { def this() = this({ println("  made: java.util.RandomAccess"); 1 }) }
object O_RandomAccess extends S_RandomAccess() with java.util.RandomAccess
object Main {
  def main(args: Array[String]): Unit = {
    println("a class: java.util.ArrayList"); O_ArrayList
    println("a class: java.util.HashMap"); O_HashMap
    println("a class: java.util.HashSet"); O_HashSet
    println("a class: java.util.ArrayDeque"); O_ArrayDeque
    println("a class: java.util.TreeSet"); O_TreeSet
    println("a class: java.lang.Throwable"); O_Throwable
    println("a class: java.lang.Exception"); O_Exception
    println("a class: java.lang.RuntimeException"); O_RuntimeException
    println("a class: java.util.concurrent.atomic.AtomicInteger"); O_AtomicInteger
    println("a class: java.io.InputStream"); O_InputStream
    println("a class: java.io.OutputStream"); O_OutputStream
    println("a class: java.io.Writer"); O_Writer
    println("a class: java.lang.ClassLoader"); O_ClassLoader
    println("a class: java.util.concurrent.ConcurrentLinkedQueue"); O_ConcurrentLinkedQueue
    println("an interface: java.lang.Runnable"); O_Runnable
    println("an interface: java.lang.Comparable"); O_Comparable
    println("an interface: java.lang.AutoCloseable"); O_AutoCloseable
    println("an interface: java.io.Closeable"); O_Closeable
    println("an interface: java.util.function.Supplier"); O_Supplier
    println("an interface: java.util.Comparator"); O_Comparator
    println("an interface: java.lang.Iterable"); O_Iterable
    println("an interface: java.io.Serializable"); O_Serializable
    println("an interface: java.lang.Cloneable"); O_Cloneable
    println("an interface: java.util.RandomAccess"); O_RandomAccess
    println("end")
  }
}
