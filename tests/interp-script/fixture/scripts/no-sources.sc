//> using dep dev.zio::zio:2.1.26
// A dependency and no source of the build: the owner named with --module.
println(zio.Chunk(1, 2, 3).size)
