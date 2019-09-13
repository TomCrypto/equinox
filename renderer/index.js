import('./pkg/webgl').catch(console.error).then(gl => {
  if (!gl) {
    throw "fail"
  }

  window.addEventListener("load", () => {
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

    document.getElementById("cat").addEventListener("click", () => {
      fetch(new Request('pkg/cat-bvh.bin')).then(bvhResponse => {
        fetch(new Request('pkg/cat-tri.bin')).then(triResponse => {
          bvhResponse.arrayBuffer().then(bvh => {
            triResponse.arrayBuffer().then(tri => {
              runner.set_bvh_data(new Uint8Array(bvh))
              runner.set_tri_data(new Uint8Array(tri))
            })
          })
        })
      })
    });

    document.getElementById("buddha").addEventListener("click", () => {
      fetch(new Request('pkg/buddha-bvh.bin')).then(bvhResponse => {
        fetch(new Request('pkg/buddha-tri.bin')).then(triResponse => {
          bvhResponse.arrayBuffer().then(bvh => {
            triResponse.arrayBuffer().then(tri => {
              runner.set_bvh_data(new Uint8Array(bvh))
              runner.set_tri_data(new Uint8Array(tri))
            })
          })
        })
      })
    });

    canvas.addEventListener("mousemove", event => {
      if (!moving) {
        return
      }

      runner.move_camera(-event.movementX * 0.001, -event.movementY * 0.001, Math.PI / 3)
    })

    canvas.addEventListener("wheel", event => {
      runner.zoom(Math.pow(1.1, 0.01 * event.deltaY))
    })

    canvas.addEventListener("mousedown", _ => {
      canvas.requestPointerLock()
      moving = true
    })

    canvas.addEventListener("mouseup", _ => {
      document.exitPointerLock()
      moving = false
    })

    const renderLoop = () => {
      runner.refine()
      runner.render()

      window.requestAnimationFrame(renderLoop)
    }
  
    window.requestAnimationFrame(renderLoop)
  })
})
