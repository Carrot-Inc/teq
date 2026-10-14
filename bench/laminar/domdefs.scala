//> using file metaProject.scala
// Laminar's DOM definitions as teq runs them in the place of sbt's (overlay/LaminarTeq.scala): the build's own
// project/DomDefsGenerator.scala, which build.sbt's `preload` runs when sbt loads the build (its `onLoad`, outside
// the generators sbt knows of), over Scala DOM Types at the version the meta-build names, its Scala 3 jar on the
// class path; it writes the checked-in files of src/main/scala/com/raquo/laminar. From the build's root:
//
//   teq interp bench/laminar/domdefs.scala project/DomDefsGenerator.scala project/project/ProjectVersions.scala \
//     --classpath domtypes_3-<version>.jar -- [dir]
//
// `generate`, not `cachedGenerate`: the cache (.downloads/domtypes.version) is sbt's record of its last run, and
// teq's of its own is the command's inputs. `dir`, the directory a TeqCommand is given to write sources into, stays
// empty: the outputs are the checked-in files, which the configuration's source directories hold.
@main def laminarDomDefs(args: String*): Unit =
  metaProject.BuildInfo.scalaDomTypesVersion = ProjectVersions.ScalaDomTypes
  DomDefsGenerator.generate()
