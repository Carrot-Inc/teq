package dev.teq.sbt

import java.io.File
import java.nio.file.Path
import scala.annotation.nowarn
import scala.util.control.NonFatal
import scala.collection.compat.*

import sbt.*
import sbt.Keys.*

import com.typesafe.sbt.SbtNativePackager.Universal
import com.typesafe.sbt.packager.Keys.{executableScriptName, packageName, stagingDirectory}
import com.typesafe.sbt.packager.PluginCompat
import com.typesafe.sbt.packager.archetypes.JavaAppPackaging.autoImport.{bundledJvmLocation, scriptClasspath, scriptClasspathOrdering}
import com.typesafe.sbt.packager.archetypes.scripts.BashStartScriptPlugin.autoImport.{bashScriptConfigLocation, bashScriptDefines, bashScriptExtraDefines, bashScriptTemplateLocation}
import com.typesafe.sbt.packager.docker.DockerPlugin.autoImport.{Docker, dockerExposedPorts, dockerGroupLayers, dockerLayerGrouping, dockerPackageMappings}
import com.typesafe.sbt.packager.linux.LinuxPlugin.autoImport.defaultLinuxInstallLocation

/** What sbt-native-packager's settings say of a project's Docker stage, read without a build:
  * the names, where the stage is written, and the layer of each file by its path in the image.
  * `layer` takes the file, its path and whether it is a jar of the build's projects, which
  * native-packager's own grouping puts with the start scripts. */
final case class NativeStage(
  packageName: String,
  scriptName: String,
  stagingDirectory: File,
  installLocation: String,
  scriptsDirectory: File,
  exposedPorts: Seq[Int],
  layer: (File, String, Boolean) => Either[String, Option[Int]],
  refusals: Seq[String],
)

/** sbt-native-packager's Docker stage for the export's `stage` block (native-packager 1.11's
  * rules). native-packager is a dependency of the plugin's compile alone: this object is reached
  * only for a project that enables `JavaAppPackaging` and its `DockerPlugin`. */
private[sbt] object DockerStage {
  /** The plugins that change the stage in ways `teq stage` does not reproduce. */
  private val Unsupported = Seq(
    "com.typesafe.sbt.packager.archetypes.jar.ClasspathJarPlugin",
    "com.typesafe.sbt.packager.archetypes.jar.LauncherJarPlugin",
    "com.typesafe.sbt.packager.archetypes.JavaServerAppPackaging",
    "com.typesafe.sbt.packager.archetypes.scripts.AshScriptPlugin",
    "com.typesafe.sbt.packager.archetypes.jlink.JlinkPlugin",
  )

  /** The layers of `dockerGroupLayers`: native-packager's own grouping, rebuilt here since its
    * task reads the packaged jars, which take a build, unless the build sets the function; then
    * the build's, evaluated when it reads settings alone and refused otherwise. */
  val of: Def.Initialize[Task[NativeStage]] = Def.taskDyn {
    val structure = buildStructure.value
    val data = settingsData.value
    val root = Export.rootOf((LocalRootProject / baseDirectory).value)
    val project = thisProjectRef.value
    val inDocker = Scope(Select(project), Select(ConfigKey(Docker.name)), Zero, Zero)
    val inProject = Scope(Select(project), Zero, Zero, Zero)
    Seq(inDocker, inProject).find(scope => Export.setByBuild(structure, root, scope, dockerGroupLayers.key)) match {
      case None => native(Def.task(Right(None)))
      case Some(scope) =>
        Export.taskDependencies(structure, data, scope, dockerGroupLayers.key) match {
          case Nil => native(Def.task(Right(Some((Docker / dockerGroupLayers).value))))
          case tasks => native(Def.task(Left(s"the build's dockerGroupLayers reads ${tasks.mkString(", ")}, which the export does not run, since it may take a build: make it a function of the paths alone")))
        }
    }
  }

  @nowarn("cat=deprecation")
  private def native(function: Def.Initialize[Task[Either[String, Option[PartialFunction[(PluginCompat.FileRef, String), Int]]]]]): Def.Initialize[Task[NativeStage]] = Def.task {
    val built = function.value
    val converter = fileConverter.value
    val structure = buildStructure.value
    val root = Export.rootOf((LocalRootProject / baseDirectory).value)
    val project = thisProjectRef.value
    val name = project.project
    val install = (Docker / defaultLinuxInstallLocation).value
    val legacy = dockerLayerGrouping.value
    val templates = sourceDirectory.value / "templates"
    val sources = Seq("Universal / sourceDirectory" -> (Universal / sourceDirectory).value, "Docker / sourceDirectory" -> (Docker / sourceDirectory).value)
    val plugins = thisProject.value.autoPlugins.map(_.label).toSet

    /** A file a reason names, as the lock records it on every machine and in every checkout:
      * relative to the build's root, and outside it by the setting that names it. */
    def named(file: File, setting: String) =
      if (file.toPath.toAbsolutePath.normalize.startsWith(root)) Export.relativeTo(root, file) else s"$setting, outside the build's root,"
    def setHere(key: AttributeKey[?], config: Option[String] = None) =
      Export.setInBuild(structure, root, project, key, scope => config.forall(c => scope.config == Select(ConfigKey(c))))
    val refusals =
      Unsupported.filter(plugins).map(p => s"$name: ${p.split('.').last} changes the Docker stage in ways teq does not reproduce") ++
        Seq(
          scriptClasspath.key -> None,
          scriptClasspathOrdering.key -> None,
          bashScriptExtraDefines.key -> None,
          bashScriptDefines.key -> None,
          bashScriptConfigLocation.key -> None,
          bashScriptTemplateLocation.key -> None,
          bundledJvmLocation.key -> None,
          dockerPackageMappings.key -> None,
          mappings.key -> Some(Universal.name),
          mappings.key -> Some(Docker.name),
          javaOptions.key -> Some(Universal.name),
        ).collect { case (key, config) if setHere(key, config) =>
          s"$name: the build sets ${config.fold("")(_ + " / ")}${key.label}, which changes the Docker stage in ways teq does not reproduce"
        } ++
        sources.collect { case (setting, dir) if dir.isDirectory && (dir ** "*").get().exists(_.isFile) =>
          s"$name: ${named(dir, setting)} holds files native-packager stages, which teq does not"
        } ++
        Option.when((templates / "bash-template").isFile)(s"$name: ${named(templates / "bash-template", "the bash-template of sourceDirectory / templates")} replaces the start script, which teq writes itself")

    val layer: (File, String, Boolean) => Either[String, Option[Int]] = built match {
      case Left(why) => (_, _, _) => Left(why)
      case Right(Some(function)) =>
        (file, path, _) =>
          try Right(function.lift((PluginCompat.toFileRef(file)(using converter), path)))
          // The exception by its class: its message may name a file of this machine, and the
          // reason is recorded in the lock.
          catch { case NonFatal(e) => Left(s"the build's dockerGroupLayers fails on $path (${e.getClass.getName}): it reads the file, which exists only after a build") }
      case Right(None) =>
        (_, path, artifact) =>
          Right(legacy(path).orElse {
            if (artifact || path.startsWith(s"$install/bin/")) Some(4)
            else if (path.startsWith(s"$install/jre/")) Some(3)
            else if (path.startsWith(s"$install/lib/")) Some(2)
            else if (path.startsWith(s"$install/conf/")) Some(1)
            else None
          })
    }

    NativeStage(
      packageName = (Docker / packageName).value,
      scriptName = executableScriptName.value,
      stagingDirectory = (Docker / stagingDirectory).value,
      installLocation = install,
      scriptsDirectory = (Universal / target).value / "scripts",
      exposedPorts = dockerExposedPorts.value,
      layer = layer,
      refusals = refusals,
    )
  }
}
