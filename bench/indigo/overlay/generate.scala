// Indigo's generated sources, as its Mill build has the plugin write them (MillIndigo's generatedSources over each
// module's indigoOptions and indigoGenerators, <module>/package.mill, written out here as the build declares them),
// run by teq as the overlay's generator commands (teq.mill.json): the plugin's generator files
// (../indigo-plugin/indigo-plugin/src/indigoplugin/) are this program's sources as they are, beside os.scala for the
// part of os-lib they call and fontgen.scala for the font generator, whose sheet the plugin draws with AWT.
//
// teq interp generate.scala os.scala fontgen.scala <the plugin's files> -- sandbox|physics|perf|shader <output>
import indigoplugin.*

@main def generate(args: String*): Unit =
  if args.length != 2 then
    System.err.println("usage: generate.scala os.scala fontgen.scala <plugin files> -- sandbox|physics|perf|shader <output>")
    sys.exit(2)
  // The directory exists, as Mill's `Task.dest` does for the build's own run; the workspace is the build's directory.
  val dest = os.Path(java.nio.file.Files.createDirectories(java.nio.file.Paths.get(args(1)).toAbsolutePath.normalize))
  val workspace = os.pwd
  val module = args(0)
  val assets = workspace / module / "assets"
  def options(title: String, width: Int, height: Int): IndigoOptions =
    IndigoOptions.defaults.withTitle(title).withWindowSize(width, height).withBackgroundColor("black")
  val (indigoOptions, generators) = module match
    case "sandbox" =>
      (
        options("Sandbox", 800, 600),
        IndigoGenerators("example")
          .generateConfig("Config")
          .embedFont(
            "TestFont",
            assets / "fonts" / "VCR_OSD_MONO_1.001.ttf",
            FontOptions("test font", 16, CharSet.ASCII).withColor(RGB.White).noAntiAliasing,
            assets / "generated"
          )
      )
    case "physics" =>
      (
        options("Physics", 800, 600).excludeAssets {
          case p if p.endsWith(os.RelPath.rel / ".DS_Store") => true
          case _                                             => false
        },
        IndigoGenerators("example")
          .generateConfig("Config")
          .embedFont(
            moduleName = "PixelatedFont",
            font = workspace / "physics" / "generator-data" / "pixelated.ttf",
            fontOptions = FontOptions(
              fontKey = "Pixelated",
              fontSize = 32,
              charSet = CharSet.ASCII,
              color = RGB.White,
              antiAlias = false,
              layout = FontLayout.default
            ),
            imageOut = assets / "generated"
          )
          .listAssets("Assets")
      )
    case "perf" => (options("Perf", 800, 600), IndigoGenerators("example").generateConfig("Config"))
    case "shader" => (options("Shader", 450, 450), IndigoGenerators("example").generateConfig("Config"))
    case other =>
      System.err.println(s"no module $other")
      sys.exit(2)
  generators.toSourcePaths(indigoOptions, assets, dest)
