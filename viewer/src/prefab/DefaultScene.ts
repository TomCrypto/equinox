export default {
  name: "[Prefab] Default Scene",
  json: {
    metadata: {
      name: "[Prefab] Default Scene"
    },
    camera: {
      position: [9.164212, 4.2127805, 10.355862],
      direction: [-0.63988286, -0.2624869, -0.7222538],
      up_vector: [0, 1, 0],
      aperture: {
        type: "circle",
        radius: 0
      },
      focal_distance: 1,
      focal_length: 0.06,
      film_height: 0.024
    },
    raster: {
      width: 1280,
      height: 720,
      filter: {
        type: "blackman-harris"
      }
    },
    instance_list: {
      "matte-sphere": {
        geometry: "sphere",
        material: "matte",
        parameters: {
          x: -2,
          y: 0.81,
          z: 0
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true
      },
      "glass-pane": {
        geometry: "pane",
        material: "glass-pane",
        parameters: {
          x: 3.02,
          y: 1.27,
          z: 0
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true
      },
      "glass-sphere": {
        geometry: "two-spheres",
        material: "copper",
        parameters: {
          x: 1,
          y: 0.81,
          z: 0
        },
        photon_receiver: false,
        sample_explicit: true,
        visible: true
      },
      ground: {
        geometry: "ground",
        material: "lambertian",
        parameters: {},
        photon_receiver: true,
        sample_explicit: true,
        visible: true
      },
      "test-sphere": {
        geometry: "cube",
        material: "gold",
        parameters: {
          x: -2,
          y: 0.81,
          z: 2.1
        },
        photon_receiver: false,
        sample_explicit: true,
        visible: true
      }
    },
    geometry_list: {
      cube: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "round",
          radius: 0.1,
          child: {
            type: "cuboid",
            dimensions: [0.699, 0.699, 0.699]
          }
        }
      },
      ground: {
        type: "cuboid",
        dimensions: [3, 0.01, 3]
      },
      pane: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "round",
          radius: 0.02,
          child: {
            type: "cuboid",
            dimensions: [0, 1.25, 3]
          }
        }
      },
      sphere: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "sphere",
          radius: 0.799
        }
      },
      "two-spheres": {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "union",
          children: [
            {
              type: "sphere",
              radius: 0.799
            },
            {
              type: "translate",
              translation: [0.8, 0.7, 0],
              child: {
                type: "sphere",
                radius: 0.4
              }
            },
            {
              type: "translate",
              translation: [-0.8, 0.8, 0],
              child: {
                type: "sphere",
                radius: 0.6
              }
            }
          ]
        }
      }
    },
    material_list: {
      copper: {
        type: "phong",
        albedo: [0.955, 0.637, 0.538],
        shininess: 150
      },
      matte: {
        type: "oren-nayar",
        albedo: [0.25, 0.75, 0.25],
        roughness: 1
      },
      "glass-pane": {
        type: "dielectric",
        internal_refractive_index: 1.65,
        external_refractive_index: 1,
        internal_extinction_coefficient: [1e-9, 0.000002, 0.000004],
        external_extinction_coefficient: [0, 0, 0],
        base_color: [1, 1, 1]
      },
      gold: {
        type: "phong",
        albedo: [1, 0.766, 0.336],
        shininess: 1200
      },
      lambertian: {
        type: "lambertian",
        albedo: [0.5, 0.5, 0.5]
      }
    },
    environment_map: null,
    environment: {
      type: "solid",
      tint: [1, 1, 1]
    },
    display: {
      exposure: 0,
      saturation: 1,
      camera_response: null
    },
    aperture: null,
    integrator: {
      hash_table_bits: 18,
      photons_per_pass: 2000000,
      photon_rate: 0.5,
      max_search_radius: 0.1,
      min_search_radius: 0.005,
      alpha: 0.7,
      max_scatter_bounces: 10,
      max_gather_bounces: 9
    }
  },
  assets: [],
  thumbnail: "prefab-default-scene.jpg",
  timestamp: ""
};
