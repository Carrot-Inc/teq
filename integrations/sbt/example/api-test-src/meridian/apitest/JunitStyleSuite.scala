package meridian.apitest

import org.junit.Test

// Found by its `@Test` method through the JUnit framework's annotation fingerprint: the
// analysis reports the annotations of a class's public methods for zinc's discovery. Not run
// by the corpus's checks (build.sbt filters it): teq writes no annotation attributes into
// class files yet, so JUnit itself would find no `@Test` in the class.
class JunitStyleSuite:
  @Test def check(): Unit = assert(1 + 1 == 2)
