package harness

import java.nio.file.{Files, Paths}

class HarnessSuite extends munit.FunSuite:
  test("runs as Mill runs it: a sandbox of its own, empty, Mill's variables, the run class path") {
    val cwd = Paths.get("").toAbsolutePath
    assertEquals(Files.list(cwd).count(), 0L, cwd.toString)
    Files.writeString(cwd.resolve("written-by-a-run.txt"), "x")
    val resources = Paths.get(sys.env("MILL_TEST_RESOURCE_DIR"))
    assertEquals(resources, Paths.get(sys.env("MILL_WORKSPACE_ROOT")).resolve("harness/test/resources"))
    assertEquals(Files.readString(resources.resolve("fixture.txt")).trim, "a resource")
    assert(getClass.getResource("/fixture.txt") != null)
    // slf4j-nop, on harness's run class path alone (runMvnDeps), names its provider.
    assert(getClass.getResource("/META-INF/services/org.slf4j.spi.SLF4JServiceProvider") != null)
  }
  test("the library") {
    assertEquals(Harness.name, "harness")
    assert(Harness.line > 0)
  }
