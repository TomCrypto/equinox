import('./pkg/webgl').catch(console.error).then(gl => {
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

  fetch(new Request('pkg/cat-bvh.bin')).then(bvhResponse => {
    fetch(new Request('pkg/cat-tri.bin')).then(triResponse => {
      bvhResponse.arrayBuffer().then(bvh => {
        triResponse.arrayBuffer().then(tri => {
          cat_object = runner.add_object(new Uint8Array(bvh), new Uint8Array(tri))
        })
      })
    })
  })

  fetch(new Request('pkg/buddha-bvh.bin')).then(bvhResponse => {
    fetch(new Request('pkg/buddha-tri.bin')).then(triResponse => {
      bvhResponse.arrayBuffer().then(bvh => {
        triResponse.arrayBuffer().then(tri => {
          buddha_object = runner.add_object(new Uint8Array(bvh), new Uint8Array(tri))
        })
      })
    })
  })

  document.getElementById("cat").addEventListener("click", () => {
    if (cat_object != -1) {
      runner.add_instance(cat_object)
    }
  });

  document.getElementById("buddha").addEventListener("click", () => {
    if (buddha_object != -1) {
      runner.add_instance(buddha_object)
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
        runner.set_camera_aperture(0.001)
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
        runner.move_camera(-dx, -dy)
      }

      runner.refine()
      runner.render()

      window.requestAnimationFrame(renderLoop)
    } catch (e) {
      console.log("ERROR:", e)
    }
  }

  window.requestAnimationFrame(renderLoop)
})
