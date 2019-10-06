async function fetch_bytes(url) {
  return (await fetch(new Request(url))).arrayBuffer();
}

import('../pkg/equinox').catch(console.error).then(async gl => {
  if (!gl) {
    throw "fail"
  }

  gl.initialize_logging()

  const canvas = document.getElementsByTagName("canvas")[0]

  let moving = false

  canvas.width = canvas.clientWidth
  canvas.height = canvas.clientHeight

  const context = canvas.getContext("webgl2", {
    alpha: false,
    antialias: false,
    depth: false,
    premultipliedAlpha: false,
    stencil: false,
  })

  runner = new gl.WasmRunner(context)

  runner.set_dimensions(canvas.width, canvas.height)
  document.getElementById("resolution").innerText = `${canvas.width} × ${canvas.height}`

  // add materials

  /*let white_mat = runner.add_material(0, 0.75, 0.75, 0.75)
  let red_mat = runner.add_material(0, 0.75, 0.25, 0.25)
  let green_mat = runner.add_material(0, 0.25, 0.75, 0.25)
  let blue_mat = runner.add_material(0, 0.25, 0.25, 0.75)
  let yellow_mat = runner.add_material(0, 0.75, 0.75, 0.25)
  let mirror = runner.add_material(1, 0, 0, 0)
  let light = runner.add_material(2, 20.0, 0.0, 0.0)
  let furnace = runner.add_material(0, 0.18, 0.18, 0.18)*/

  canvas.addEventListener("webglcontextlost", e => {
    runner.context_lost();

    e.preventDefault()
  })

  ctx = context.getExtension("WEBGL_lose_context");

  document.getElementById("lose").addEventListener("click", () => {
    ctx.loseContext();
  });

  document.getElementById("restore").addEventListener("click", () => {
    ctx.restoreContext();
  });

  let aperture_r = new Float32Array(await fetch_bytes("assets/r_spectrum.dat"))
  let aperture_g = new Float32Array(await fetch_bytes("assets/g_spectrum.dat"))
  let aperture_b = new Float32Array(await fetch_bytes("assets/b_spectrum.dat"))
  let aperture = new Uint8Array(await fetch_bytes("assets/aperture.bin"))
  runner.set_aperture_data(aperture, 1024, 1024)

  let envmap_data = new Float32Array(await fetch_bytes("assets/blue_grotto_4k.raw"))
  runner.set_envmap(envmap_data, 4096, 2048)

  // 5 or 7 or 8
  //let envmap_data = new Float32Array(await fetch_bytes("assets/envmap5.dat"))
  //runner.set_envmap(envmap_data, 8192, 4096)

  runner.set_camera_position(0.5, 4, -12)

  runner.setup_test_scene();

  document.getElementById("cat").addEventListener("click", () => {
    runner.add_instance(cat_object, 0, 0, 0, 1, [blue_mat])
  });

  let offset = 0

  document.getElementById("buddha").addEventListener("click", () => {
    runner.add_instance(buddha_object, offset, 0, 0, 300, [mirror])

    offset += 1.0
  });

  document.getElementById("cornell").addEventListener("click", () => {
    // runner.add_instance(cornell_object, 0, 0, 0, 1, [red_mat])
  });

  document.getElementById("sphere").addEventListener("click", () => {
    runner.add_instance(sphere_object, 0, 0, 0, 5.0, [furnace])
  });

  document.getElementById("remove-first").addEventListener("click", () => {
    if (runner.instance_count() != 0) {
      runner.remove_instance(0)
    }
  });

  document.getElementById("remove-last").addEventListener("click", () => {
    if (runner.instance_count() != 0) {
      runner.remove_instance(runner.instance_count() - 1)
    }
  });

  let apertureSize = document.getElementById("aperture-size")
  apertureSize.addEventListener("input", () => {
    runner.set_camera_aperture(apertureSize.value / 10000)
  })

  let focalDistance = document.getElementById("focal-distance")
  focalDistance.addEventListener("input", () => {
    runner.set_focal_distance(focalDistance.value / 100)
  })

  let focalLength = document.getElementById("focal-length")
  focalLength.addEventListener("input", () => {
    runner.set_focal_length(focalLength.value / 1000)
  })

  let exposure = document.getElementById("exposure")
  exposure.addEventListener("input", () => {
    runner.set_display_exposure(exposure.value / 1000)
  })

  let saturation = document.getElementById("saturation")
  saturation.addEventListener("input", () => {
    runner.set_display_saturation(saturation.value / 10000)
  })

  let camera_response = document.getElementById("camera-response")
  camera_response.addEventListener("change", () => {
    runner.set_camera_response(camera_response.value)
  })

  let angleX = 4.758
  let angleY = 1.238
  let pressed = {}

  let x = Math.sin(angleY) * Math.cos(angleX);
  let z = Math.sin(angleY) * Math.sin(angleX);
  let y = Math.cos(angleY);

  runner.set_camera_direction(x, y, z)

  canvas.addEventListener("mousemove", event => {
    if (!moving) {
      return
    }

    angleX += -event.movementX * 0.001;
    angleY += -event.movementY * 0.001;

    if (angleY > Math.PI - 0.01) {
      angleY = Math.PI - 0.01
    }

    if (angleY < 0.01) {
      angleY = 0.01
    }

    let x = Math.sin(angleY) * Math.cos(angleX);
    let z = Math.sin(angleY) * Math.sin(angleX);
    let y = Math.cos(angleY);

    runner.set_camera_direction(x, y, z)
    samples = 0
  })

  canvas.addEventListener("mousedown", _ => {
    canvas.requestPointerLock()
    moving = true
  })

  canvas.addEventListener("mouseup", _ => {
    document.exitPointerLock()
    moving = false
  })

  window.addEventListener("keydown", e => {
    pressed[e.key] = true
  })

  window.addEventListener("keyup", e => {
    delete pressed[e.key]
  })

  let start = performance.now()
  let times = []
  let updateEMA = new ExponentialMovingAverage(0.12)
  let renderEMA = new ExponentialMovingAverage(0.08)
  let refineEMA = new ExponentialMovingAverage(0.08)
  let samples = 0

  const renderLoop = () => {
    let oldStart = start
    start = performance.now()

    addToSeries(times, 30, start - oldStart)

    document.getElementById("frame-rate").innerText = `${(1000 / seriesAverage(times)).toFixed(0)} FPS`
    
    try {
      let dx = 0
      let dy = 0

      if (pressed['q'] == true) {
        runner.set_camera_aperture(0.1)
        samples = 0
      }

      if (pressed['w'] === true) {
        dx += 0.02
      }

      if (pressed['s'] === true) {
        dx -= 0.02
      }

      if (pressed['a'] === true) {
        dy -= 0.02
      }

      if (pressed['d'] === true) {
        dy += 0.02
      }

      if (dx != 0.0 || dy != 0.0) {
        runner.move_camera(-dx * 10, -dy * 10)
        samples = 0
      }

      if (canvas.width != canvas.clientWidth || canvas.height != canvas.clientHeight) {
        canvas.width = canvas.clientWidth
        canvas.height = canvas.clientHeight
        runner.set_dimensions(canvas.width, canvas.height)

        document.getElementById("resolution").innerText = `${canvas.width} × ${canvas.height}`
      }
      
      let refineCount = 1

      if (isFinite(refineEMA.average()) && isFinite(renderEMA.average())) {
        refineCount = Math.floor((1000000.0 / 60.0 - 2000.0 - renderEMA.average()) / refineEMA.average())
      }

      refineCount = 1 // Math.min(40, Math.max(refineCount, 1))

      let now = performance.now()
      runner.update()
      let elapsed = performance.now() - now
      updateEMA.append(elapsed)
      for (let i = 0; i < refineCount; ++i) {
        runner.refine()
        samples += 1

        let refineTime = runner.get_refine_frame_time()

        if (refineTime !== 0) {
          refineEMA.append(refineTime)
        }
      }
      runner.render()

      let renderTime = runner.get_render_frame_time()
      

      if (renderTime !== 0) {
        renderEMA.append(renderTime)
      }

      

      let updateAvg = displayTime(updateEMA.average())
      let refineAvg = displayTime(refineEMA.average() / 1000.0)
      let renderAvg = displayTime(renderEMA.average() / 1000.0)

      document.getElementById("frame-info").innerText = ` [update: ${updateAvg}] ➜ [refine: ${refineAvg} × ${refineCount.toFixed(0).padStart(2, ' ')}] ➜ [render: ${renderAvg}]`
      document.getElementById("frame-rate").innerText = `${(1000 / seriesAverage(times)).toFixed(0)} fps`
      document.getElementById("sample-count").innerText = `${samples} samples`

      window.requestAnimationFrame(renderLoop)
    } catch (e) {
      console.log("ERROR:", e)
    }
  }

  window.requestAnimationFrame(renderLoop)
})

function displayTime(milliseconds) {
  if (!isFinite(milliseconds)) {
    return '---- ms'
  }

  if (milliseconds <= 0.099) {
    return `${(milliseconds * 1000.0).toFixed(0).padStart(4, ' ')} μs`
  }

  if (milliseconds <= 99) {
    return `${(milliseconds).toFixed(1).padStart(4, ' ')} ms`
  }

  return `${(milliseconds).toFixed(0).padStart(4, ' ')} ms`
}

function addToSeries(series, max, value) {
  let avg = seriesAverage(series)

  series.push(value)

  while (series.length > max) {
    series.shift()
  }
}

function seriesAverage(series) {
  let average = 0

  for (value of series) {
    average += value / series.length
  }

  return average
}

class ExponentialMovingAverage {
  constructor (alpha) {
    this.alpha = alpha
    this.value = NaN
  }

  append(value) {
    if (!isFinite(this.value)) {
      this.value = value
    } else {
      this.value = this.value * (1 - this.alpha) + value * this.alpha
    }
  }

  average() {
    return this.value
  }
}
