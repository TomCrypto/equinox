async function fetch_bytes(url) {
  return (await fetch(new Request(url))).arrayBuffer();
}

async function load_model_data(name) {
  hierarchy_data = new Uint8Array(await fetch_bytes(`pkg/${name}.hierarchy.bin`))
  triangle_data = new Uint8Array(await fetch_bytes(`pkg/${name}.triangle.bin`))
  position_data = new Uint8Array(await fetch_bytes(`pkg/${name}.position.bin`))
  normal_data = new Uint8Array(await fetch_bytes(`pkg/${name}.mapping.bin`))

  return [hierarchy_data, triangle_data, position_data, normal_data]
}

import('./pkg/webgl').catch(console.error).then(async gl => {
  if (!gl) {
    throw "fail"
  }

  const canvas = document.getElementsByTagName("canvas")[0]

  let moving = false

  canvas.width = canvas.clientWidth
  canvas.height = canvas.clientHeight

  const runner = new gl.WasmRunner(canvas.getContext("webgl2", {
    alpha: false,
    antialias: false,
    depth: false,
    premultipliedAlpha: false,
    stencil: false,
  }))

  runner.set_dimensions(canvas.width, canvas.height)

  canvas.addEventListener("webglcontextlost", e => {
    runner.context_lost();

    e.preventDefault()
  })

  ctx = runner.context().getExtension("WEBGL_lose_context");

  document.getElementById("lose").addEventListener("click", () => {
    ctx.loseContext();
  });

  document.getElementById("restore").addEventListener("click", () => {
    ctx.restoreContext();
  });

  let cat_object = -1
  let buddha_object = -1

  let [bvh, tri, position, normal] = await load_model_data('cat')
  cat_object = runner.add_object(bvh, tri, position, normal, 2, -484.04044, 7.148789, -72.22099, 277.95947, 338.37366, 72.22315)
  let [bvh2, tri2, position2, normal2] = await load_model_data('buddha')
  buddha_object = runner.add_object(bvh2, tri2, position2, normal2, 1, -0.188615, -0.445945, -0.224346, 0.222054, 0.554055, 0.186807)

  document.getElementById("cat").addEventListener("click", () => {
    if (cat_object != -1) {
      runner.add_instance(cat_object, 0, 0, 0)
    }
  });

  let offset = 0

  document.getElementById("buddha").addEventListener("click", () => {
    if (buddha_object != -1) {
      runner.add_instance(buddha_object, offset, 0, 0)

      offset += 1.0
    }
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

  document.getElementById("push-up").addEventListener("click", () => {
    if (runner.instance_count() != 0) {
      runner.move_instance_up(0, 10.0);
    }
  });

  let apertureSize = document.getElementById("aperture-size")
  apertureSize.addEventListener("input", () => {
    runner.set_camera_aperture(apertureSize.value / 10000)
  })

  let focalDistance = document.getElementById("focal-distance")
  focalDistance.addEventListener("input", () => {
    runner.set_focal_distance(focalDistance.value / 1000)
  })

  let focalLength = document.getElementById("focal-length")
  focalLength.addEventListener("input", () => {
    runner.set_focal_length(focalLength.value / 1000)
  })

  let angleX = Math.PI / 2
  let angleY = Math.PI / 2
  let pressed = {}

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

  const renderLoop = () => {
    try {
      let dx = 0
      let dy = 0

      if (pressed['q'] == true) {
        runner.set_camera_aperture(0.1)
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
        runner.move_camera(-dx * 5, -dy * 5)
      }

      if (canvas.width != canvas.clientWidth || canvas.height != canvas.clientHeight) {
        canvas.width = canvas.clientWidth
        canvas.height = canvas.clientHeight
        runner.set_dimensions(canvas.width, canvas.height)
      }

      runner.update()
      runner.refine()
      runner.render()

      window.requestAnimationFrame(renderLoop)
    } catch (e) {
      console.log("ERROR:", e)
    }
  }

  window.requestAnimationFrame(renderLoop)
})
