println($u0070robe_sc.args$opt.isDefined)
// A script's wrapper object is found by the encoded key of its name, which decoding never reaches (dotty's
// PackageScope keys by name.mangled, which encodes): `$u0070robe_sc` encodes to itself, not to `probe_sc`, as
// scalac 3.8.4 refuses it in Scala CLI's generated file.
// expect: probe.sc:1:9: error: not found: $u0070robe_sc
