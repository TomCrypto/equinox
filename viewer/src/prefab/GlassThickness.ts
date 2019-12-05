export default {
  name: "[Prefab] Glass Thickness",
  json: {
    metadata: {
      name: "[Prefab] Glass Thickness"
    },
    camera: {
      position: [-13.5, 1.5, 0],
      direction: [1, 0, 0],
      up_vector: [0, 1, 0],
      aperture: {
        type: "circle",
        radius: 0
      },
      focal_distance: 1,
      focal_length: 0.07,
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
      glass1: {
        geometry: "box",
        material: "glass",
        parameters: {
          length: 0,
          x: -1,
          y: 1,
          z: 3
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [3e-8, 3e-8, 3e-8],
          refractive_index: 1.55
        },
        parent: null
      },
      glass2: {
        geometry: "box",
        material: "glass",
        parameters: {
          length: 0.5,
          x: -0.5,
          y: 1,
          z: 1
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [3e-8, 3e-8, 3e-8],
          refractive_index: 1.55
        },
        parent: null
      },
      glass3: {
        geometry: "box",
        material: "glass",
        parameters: {
          length: 1,
          x: 0,
          y: 1,
          z: -1
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [3e-8, 3e-8, 3e-8],
          refractive_index: 1.55
        },
        parent: null
      },
      glass4: {
        geometry: "box",
        material: "glass",
        parameters: {
          length: 2,
          x: 1,
          y: 1,
          z: -3
        },
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [3e-8, 3e-8, 3e-8],
          refractive_index: 1.55
        },
        parent: null
      },
      ground: {
        geometry: "ground",
        material: "lambertian",
        parameters: {},
        photon_receiver: true,
        sample_explicit: true,
        visible: true,
        medium: {
          extinction: [0, 0, 0],
          refractive_index: 1
        },
        parent: null
      }
    },
    geometry_list: {
      box: {
        type: "translate",
        translation: ["x", "y", "z"],
        child: {
          type: "round",
          radius: 0.03,
          child: {
            type: "cuboid",
            dimensions: ["length", 0.5, 0.5]
          }
        }
      },
      ground: {
        type: "cuboid",
        dimensions: [4, 0.01, 4]
      }
    },
    material_list: {
      glass: {
        type: "dielectric",
        base_color: [1, 1, 1]
      },
      lambertian: {
        type: "lambertian",
        albedo: [0.7, 0.7, 0.7]
      }
    },
    environment_map: "assets/green_point_park_8k.raw",
    environment: {
      type: "map",
      tint: [1, 1, 1],
      rotation: 3.7133625
    },
    display: {
      exposure: 0,
      saturation: 1,
      camera_response: null
    },
    aperture: null,
    integrator: {
      hash_table_bits: 18,
      photons_per_pass: 800000,
      photon_rate: 0.5,
      max_search_radius: 0.05,
      min_search_radius: 0.0025,
      alpha: 0.75,
      max_scatter_bounces: 8,
      max_gather_bounces: 8
    }
  },
  assets: ["assets/green_point_park_8k.raw"],
  thumbnail: "prefab-glass-thickness.jpg",
  timestamp: ""
};
