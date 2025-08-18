import type { Meta, StoryObj } from '@storybook/react';
import { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent } from './card';
import { Button } from './button';

const meta: Meta<typeof Card> = {
  title: 'UI/Card',
  component: Card,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component: 'A card component that groups related information with a consistent structure.',
      },
    },
  },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: () => (
    <Card className="w-[350px]">
      <CardHeader>
        <CardTitle>Create project</CardTitle>
        <CardDescription>Deploy your new project in one-click.</CardDescription>
      </CardHeader>
      <CardContent>
        <p>Card content goes here</p>
      </CardContent>
      <CardFooter className="flex justify-between">
        <Button variant="outline">Cancel</Button>
        <Button>Deploy</Button>
      </CardFooter>
    </Card>
  ),
};

export const Simple: Story = {
  render: () => (
    <Card className="w-[350px]">
      <CardHeader>
        <CardTitle>Simple Card</CardTitle>
      </CardHeader>
      <CardContent>
        <p>This is a simple card with just a title and content.</p>
      </CardContent>
    </Card>
  ),
};

export const WithDescription: Story = {
  render: () => (
    <Card className="w-[350px]">
      <CardHeader>
        <CardTitle>Notifications</CardTitle>
        <CardDescription>
          You have 3 unread messages.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          <div className="p-2 bg-muted rounded">Message 1</div>
          <div className="p-2 bg-muted rounded">Message 2</div>
          <div className="p-2 bg-muted rounded">Message 3</div>
        </div>
      </CardContent>
    </Card>
  ),
};

export const Interactive: Story = {
  render: () => (
    <Card className="w-[350px] cursor-pointer hover:shadow-lg transition-shadow">
      <CardHeader>
        <CardTitle>Interactive Card</CardTitle>
        <CardDescription>Click to interact</CardDescription>
      </CardHeader>
      <CardContent>
        <p>This card has hover effects and can be clicked.</p>
      </CardContent>
    </Card>
  ),
};