export default {
  definitions: {
    asset: {
      type: "string",
      pattern: "\\.raw$"
    },
    optionalAsset: {
      type: ["null", "string"],
      pattern: "\\.raw$"
    },
    vector3: {
      type: "array",
      minItems: 3,
      maxItems: 3,
      items: {
        type: "number"
      }
    }
  },
  $schema: "http://json-schema.org/draft-07/schema#",
  type: "object",
  properties: {
    json: {
      type: "object",
      required: [
        "camera",
        "raster",
        "instance_list",
        "geometry_list",
        "material_list",
        "environment_map",
        "environment",
        "display",
        "aperture",
        "integrator"
      ],
      properties: {
        camera: {
          type: "object",
          required: [
            "position",
            "direction",
            "up_vector",
            "aperture",
            "focal_distance",
            "focal_length",
            "film_height"
          ],
          properties: {
            position: {
              type: "object",
              required: ["x", "y", "z"],
              properties: {
                x: {
                  type: "number"
                },
                y: {
                  type: "number"
                },
                z: {
                  type: "number"
                }
              }
            },
            direction: {
              type: "object",
              required: ["x", "y", "z"],
              properties: {
                x: {
                  type: "number"
                },
                y: {
                  type: "number"
                },
                z: {
                  type: "number"
                }
              }
            },
            up_vector: {
              type: "object",
              required: ["x", "y", "z"],
              properties: {
                x: {
                  type: "number"
                },
                y: {
                  type: "number"
                },
                z: {
                  type: "number"
                }
              }
            },
            aperture: {
              type: "object",
              oneOf: [
                {
                  properties: {
                    type: {
                      type: "string",
                      enum: ["point"]
                    }
                  },
                  required: ["type"]
                },
                {
                  properties: {
                    type: {
                      type: "string",
                      enum: ["circle"]
                    },
                    radius: {
                      type: "number",
                      minimum: 0
                    }
                  },
                  required: ["type", "radius"]
                },
                {
                  properties: {
                    type: {
                      type: "string",
                      enum: ["ngon"]
                    },
                    radius: {
                      type: "number",
                      minimum: 0
                    },
                    sides: {
                      type: "number",
                      multipleOf: 1,
                      minimum: 3
                    },
                    rotation: {
                      type: "number"
                    }
                  },
                  required: ["type", "sides", "radius", "rotation"]
                }
              ]
            },
            focal_distance: {
              type: "number",
              exclusiveMinimum: 0
            },
            focal_length: {
              type: "number",
              exclusiveMinimum: 0
            },
            film_height: {
              type: "number",
              exclusiveMinimum: 0
            }
          }
        },
        raster: {
          type: "object",
          required: ["width", "height", "filter"],
          properties: {
            width: {
              type: "number",
              multipleOf: 1,
              minimum: 1,
              maximum: 8192
            },
            height: {
              type: "number",
              multipleOf: 1,
              minimum: 1,
              maximum: 8192
            },
            filter: {
              type: "object",
              required: ["type"],
              properties: {
                type: {
                  type: "string",
                  enum: ["blackman-harris", "dirac"]
                }
              }
            }
          }
        },
        instance_list: {
          type: "object",

          patternProperties: {
            "^.*$": {
              type: "object",

              properties: {
                geometry: {
                  type: "string"
                },
                material: {
                  type: "string"
                },
                parameters: {
                  type: "object",
                  patternProperties: {
                    "^.*$": {
                      type: "number"
                    }
                  },
                  additionalProperties: false
                },
                photon_receiver: {
                  type: "boolean"
                },
                sample_explicit: {
                  type: "boolean"
                },
                visible: {
                  type: "boolean"
                }
              },

              required: [
                "geometry",
                "material",
                "parameters",
                "photon_receiver",
                "sample_explicit",
                "visible"
              ]
            }
          },
          additionalProperties: false
        },
        geometry_list: {
          type: "object",

          patternProperties: {
            "^.*$": {
              type: "object" // TODO: implement JSON schema for geometries
            }
          },

          additionalProperties: false
        },
        material_list: {
          type: "object",

          patternProperties: {
            "^.*$": {
              type: "object" // TODO: implement JSON schema for materials
            }
          },

          additionalProperties: false
        },
        environment_map: {
          $ref: "#/definitions/optionalAsset"
        },
        environment: {
          type: "object",
          oneOf: [
            {
              properties: {
                type: {
                  type: "string",
                  enum: ["solid"]
                },
                tint: {
                  $ref: "#/definitions/vector3"
                }
              },
              required: ["type", "tint"]
            },
            {
              properties: {
                type: {
                  type: "string",
                  enum: ["map"]
                },
                tint: {
                  $ref: "#/definitions/vector3"
                },
                rotation: {
                  type: "number"
                }
              },
              required: ["type", "tint", "rotation"]
            }
          ]
        },
        display: {
          type: "object",
          required: ["exposure", "saturation", "camera_response"],
          properties: {
            exposure: {
              type: "number",
              minimum: -10,
              maximum: +10
            },
            saturation: {
              type: "number",
              minimum: 0,
              maximum: 1
            },
            camera_response: {
              type: "null"
            }
          }
        },
        aperture: {
          type: "null"
        },
        integrator: {
          type: "object",
          required: [
            "hash_table_bits",
            "photons_per_pass",
            "photon_rate",
            "max_search_radius",
            "min_search_radius",
            "alpha",
            "max_scatter_bounces",
            "max_gather_bounces"
          ],
          properties: {
            hash_table_bits: {
              type: "number",
              multipleOf: 1,
              minimum: 18,
              maximum: 24
            },
            photons_per_pass: {
              type: "number",
              multipleOf: 1,
              minimum: 1
            },
            photon_rate: {
              type: "number",
              minimum: 0.1,
              maximum: 0.9
            },
            max_search_radius: {
              type: "number",
              exclusiveMinimum: 0
            },
            min_search_radius: {
              type: "number",
              exclusiveMinimum: 0
            },
            alpha: {
              type: "number",
              minimum: 0,
              maximum: 1
            },
            max_scatter_bounces: {
              type: "number",
              multipleOf: 1,
              minimum: 1
            },
            max_gather_bounces: {
              type: "number",
              multipleOf: 1,
              minimum: 1
            }
          }
        }
      }
    },
    assets: {
      type: "array",
      items: {
        $ref: "#/definitions/asset"
      }
    }
  },
  required: ["json", "assets"]
};
