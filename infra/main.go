package main

import (
	"github.com/pulumi/pulumi-digitalocean/sdk/v4/go/digitalocean"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi"
	"github.com/pulumi/pulumi/sdk/v3/go/pulumi/config"
)

func main() {
	pulumi.Run(func(ctx *pulumi.Context) error {
		conf := config.New(ctx, "")

		app, err := digitalocean.NewApp(ctx, "wind-tunnel-runner-status-dashboard", &digitalocean.AppArgs{
			Spec: &digitalocean.AppSpecArgs{
				Name:   pulumi.String("wt-runner-status-dashboard"), // Full name is too long
				Region: pulumi.String("fra1"),
				Services: digitalocean.AppSpecServiceArray{
					&digitalocean.AppSpecServiceArgs{
						Name:             pulumi.String("web"),
						InstanceCount:    pulumi.Int(1),
						InstanceSizeSlug: pulumi.String("apps-s-1vcpu-1gb"),
						Git: &digitalocean.AppSpecServiceGitArgs{
							RepoCloneUrl: pulumi.String("https://github.com/holochain/wind-tunnel-runner-status-dashboard.git"),
							Branch:       pulumi.String("main"),
						},
						DockerfilePath: pulumi.String("Dockerfile"),
						HttpPort: pulumi.Int(3000),
						Envs: digitalocean.AppSpecServiceEnvArray{
							&digitalocean.AppSpecServiceEnvArgs{
								Key:   pulumi.String("NOMAD_URL"),
								Value: pulumi.String("https://nomad-server-01.holochain.org:4646"),
								Scope: pulumi.String("RUN_TIME"),
								Type:  pulumi.String("GENERAL"),
							},
							&digitalocean.AppSpecServiceEnvArgs{
								Key:   pulumi.String("NOMAD_TOKEN"),
								Value: conf.RequireSecret("nomadToken"),
								Scope: pulumi.String("RUN_TIME"),
								Type:  pulumi.String("SECRET"),
							},
							&digitalocean.AppSpecServiceEnvArgs{
								Key:   pulumi.String("NOMAD_ACCEPT_INVALID_CERT"),
								Value: pulumi.String("true"),
								Scope: pulumi.String("RUN_TIME"),
								Type:  pulumi.String("GENERAL"),
							},
							&digitalocean.AppSpecServiceEnvArgs{
								Key:   pulumi.String("BIND_ADDR"),
								Value: pulumi.String("0.0.0.0:3000"),
								Scope: pulumi.String("RUN_TIME"),
								Type:  pulumi.String("GENERAL"),
							},
							&digitalocean.AppSpecServiceEnvArgs{
								Key:   pulumi.String("UPDATE_SECONDS"),
								Value: pulumi.String("60"),
								Scope: pulumi.String("RUN_TIME"),
								Type:  pulumi.String("GENERAL"),
							},
						},
						HealthCheck: &digitalocean.AppSpecServiceHealthCheckArgs{
							HttpPath:            pulumi.String("/"),
							InitialDelaySeconds: pulumi.Int(30),
							PeriodSeconds:       pulumi.Int(10),
							TimeoutSeconds:      pulumi.Int(5),
							SuccessThreshold:    pulumi.Int(1),
							FailureThreshold:    pulumi.Int(3),
						},
					},
				},
			},
		})

		if err != nil {
			return err
		}

		ctx.Export("appUrl", app.LiveUrl)
		ctx.Export("appId", app.ID())
		ctx.Export("defaultIngress", app.DefaultIngress)

		return nil
	})
}