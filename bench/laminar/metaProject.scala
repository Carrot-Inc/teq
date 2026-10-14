// The object sbt-buildinfo writes in Laminar's meta-build (project/build.sbt): the version of Scala DOM Types,
// project/project/ProjectVersions.scala's, which domdefs.scala sets from the file the command names (an object of
// the empty package, which this package does not see). The generator reads it in `cachedGenerate` alone.
package metaProject

object BuildInfo {
  var scalaDomTypesVersion: String = ""
}
