// jars: scala-library
// std: lean scala-library
// JDK classes under packages of two segments that ct.sym holds (`com.sun.management`,
// `org.w3c.dom`), found as the JDK's where `java`, `javax`, `jdk` and `sun` are.
import com.sun.management.HotSpotDiagnosticMXBean
import java.lang.management.ManagementFactory
import org.w3c.dom.Document

object Main:
  def main(args: Array[String]): Unit =
    val server = ManagementFactory.getPlatformMBeanServer
    val bean = ManagementFactory.newPlatformMXBeanProxy(server, "com.sun.management:type=HotSpotDiagnostic", classOf[HotSpotDiagnosticMXBean])
    println(bean.getVMOption("MaxHeapSize").getName)
    val d: Document = null
    println(d == null)
