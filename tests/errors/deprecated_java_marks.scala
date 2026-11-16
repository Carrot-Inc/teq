// expect: 11:19: warning: method encode in object URLEncoder is deprecated: see corresponding Javadoc for more information.
// expect: 12:19: warning: method decode in object URLDecoder is deprecated: see corresponding Javadoc for more information.
// expect: 13:17: warning: method getId in class Thread is deprecated: see corresponding Javadoc for more information.
// expect: 14:36: warning: method isDaemon in class ThreadGroup is deprecated: see corresponding Javadoc for more information.
// expect: 15:33: warning: method setDaemon in class ThreadGroup is deprecated: see corresponding Javadoc for more information.
// expect: 5 warnings found, errors under --werror
// teq: --werror --deprecation
// The Java methods the std marks are warned of with the one text scalac gives a Java `@Deprecated`, which names
// no version.
object Marks {
  def a: String = java.net.URLEncoder.encode("a b")
  def b: String = java.net.URLDecoder.decode("a+b")
  def c: Long = Thread.currentThread().getId()
  def d(g: ThreadGroup): Boolean = g.isDaemon()
  def e(g: ThreadGroup): Unit = g.setDaemon(true)
}
